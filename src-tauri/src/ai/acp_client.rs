//! ACP Client - handles communication with AI agents via Agent Client Protocol
//!
//! This module spawns agent processes and communicates with them using ACP,
//! a JSON-RPC based protocol over stdio. Supports both one-shot requests
//! (for diff analysis) and persistent sessions (for interactive chat).

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use agent_client_protocol::{
    Agent, ClientSideConnection, ContentBlock as AcpContentBlock, Implementation,
    InitializeRequest, LoadSessionRequest, NewSessionRequest, PermissionOptionId, PromptRequest,
    ProtocolVersion, RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    Result as AcpResult, SelectedPermissionOutcome, SessionId, SessionNotification, TextContent,
};
use async_trait::async_trait;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

/// System context prepended to the first message in new sessions.
/// This guides the agent's behavior for Staged's code review use case.
const STAGED_SYSTEM_CONTEXT: &str = r#"[System Context for Staged - Code Review Assistant]

You are helping with code review in Staged, a diff viewer application. Your role is to help users understand, plan changes to, and research code in their changesets.

Output Guidelines:
- When asked to create a PLAN: produce a structured markdown document with clear objectives, step-by-step tasks, and file references
- When asked to do RESEARCH: produce a research document with summary of findings, relevant code references, and recommendations
- When answering QUESTIONS: be concise and focused on the code changes

The user is viewing a diff. Context tags like [Changeset: ...], [Viewing: ...], and [Original task: ...] provide information about what they're looking at.

---

"#;

/// Supported ACP-compatible AI agents
#[derive(Debug, Clone)]
pub enum AcpAgent {
    Goose(PathBuf),
    Claude(PathBuf),
}

impl AcpAgent {
    pub fn name(&self) -> &'static str {
        match self {
            AcpAgent::Goose(_) => "goose",
            AcpAgent::Claude(_) => "claude",
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            AcpAgent::Goose(p) => p,
            AcpAgent::Claude(p) => p,
        }
    }

    /// Get the arguments to start ACP mode
    pub fn acp_args(&self) -> Vec<&str> {
        match self {
            // Include developer extension for file/shell access, and extensionmanager
            // to allow discovering/enabling additional extensions as needed
            AcpAgent::Goose(_) => vec!["acp", "--with-builtin", "developer,extensionmanager"],
            AcpAgent::Claude(_) => vec![], // claude-code-acp runs in ACP mode by default
        }
    }
}

/// Common paths where CLIs might be installed (for GUI apps that don't inherit shell PATH)
const COMMON_PATHS: &[&str] = &[
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "/usr/bin",
    "/home/linuxbrew/.linuxbrew/bin",
];

/// Find goose CLI using login shell (to get user's PATH)
fn find_via_login_shell(cmd: &str) -> Option<PathBuf> {
    let which_cmd = format!("which {}", cmd);

    // Try zsh first (default on macOS)
    if let Ok(output) = std::process::Command::new("/bin/zsh")
        .args(["-l", "-c", &which_cmd])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(path_str) = stdout.lines().rfind(|l| !l.is_empty()) {
                let path_str = path_str.trim();
                if !path_str.is_empty() && path_str.starts_with('/') {
                    return Some(PathBuf::from(path_str));
                }
            }
        }
    }

    // Fallback to bash
    if let Ok(output) = std::process::Command::new("/bin/bash")
        .args(["-l", "-c", &which_cmd])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(path_str) = stdout.lines().rfind(|l| !l.is_empty()) {
                let path_str = path_str.trim();
                if !path_str.is_empty() && path_str.starts_with('/') {
                    return Some(PathBuf::from(path_str));
                }
            }
        }
    }

    None
}

/// Verify a command works by running it with --version
fn verify_command(path: &Path) -> bool {
    std::process::Command::new(path)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Information about an available ACP provider
#[derive(Debug, Clone, serde::Serialize)]
pub struct AcpProviderInfo {
    pub id: String,
    pub label: String,
}

/// Discover all available ACP providers on the system
pub fn discover_acp_providers() -> Vec<AcpProviderInfo> {
    let mut providers = Vec::new();

    if find_agent("goose", AcpAgent::Goose).is_some() {
        providers.push(AcpProviderInfo {
            id: "goose".to_string(),
            label: "Goose".to_string(),
        });
    }

    if find_agent("claude-code-acp", AcpAgent::Claude).is_some() {
        providers.push(AcpProviderInfo {
            id: "claude".to_string(),
            label: "Claude Code".to_string(),
        });
    }

    providers
}

/// Find a specific ACP agent by provider ID
pub fn find_acp_agent_by_id(provider_id: &str) -> Option<AcpAgent> {
    match provider_id {
        "goose" => find_agent("goose", AcpAgent::Goose),
        "claude" => find_agent("claude-code-acp", AcpAgent::Claude),
        _ => None,
    }
}

/// Find an ACP-compatible AI agent
/// Prefers Goose if available, falls back to Claude
pub fn find_acp_agent() -> Option<AcpAgent> {
    // Try Goose first (default)
    if let Some(agent) = find_agent("goose", AcpAgent::Goose) {
        return Some(agent);
    }

    // Fall back to Claude (claude-code-acp)
    find_agent("claude-code-acp", AcpAgent::Claude)
}

/// Find a specific agent by command name
fn find_agent<F>(cmd: &str, constructor: F) -> Option<AcpAgent>
where
    F: Fn(PathBuf) -> AcpAgent,
{
    // Strategy 1: Login shell which
    if let Some(path) = find_via_login_shell(cmd) {
        if verify_command(&path) {
            return Some(constructor(path));
        }
    }

    // Strategy 2: Direct command
    let direct_path = PathBuf::from(cmd);
    if verify_command(&direct_path) {
        return Some(constructor(direct_path));
    }

    // Strategy 3: Common paths
    for dir in COMMON_PATHS {
        let path = PathBuf::from(dir).join(cmd);
        if path.exists() && verify_command(&path) {
            return Some(constructor(path));
        }
    }

    None
}

/// Shared state for collecting the response
struct ResponseCollector {
    accumulated_content: Mutex<String>,
}

/// Client implementation for handling agent notifications
struct StagedAcpClient {
    collector: Arc<ResponseCollector>,
}

#[async_trait(?Send)]
impl agent_client_protocol::Client for StagedAcpClient {
    async fn request_permission(
        &self,
        args: RequestPermissionRequest,
    ) -> AcpResult<RequestPermissionResponse> {
        // Auto-approve permissions (Staged doesn't use tools that need approval)
        log::debug!("Permission requested: {:?}", args);

        let option_id = args
            .options
            .first()
            .map(|opt| opt.option_id.clone())
            .unwrap_or_else(|| PermissionOptionId::new("approve"));

        Ok(RequestPermissionResponse::new(
            RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(option_id)),
        ))
    }

    async fn session_notification(&self, notification: SessionNotification) -> AcpResult<()> {
        use agent_client_protocol::SessionUpdate;

        log::debug!(
            "[ACP Notification] session_notification: {:?}",
            notification
        );
        match &notification.update {
            SessionUpdate::AgentMessageChunk(chunk) => {
                if let AcpContentBlock::Text(text) = &chunk.content {
                    let mut accumulated = self.collector.accumulated_content.lock().await;
                    accumulated.push_str(&text.text);
                }
            }
            _ => {}
        }

        Ok(())
    }
}

/// Result of running an ACP prompt with session support
pub struct AcpPromptResult {
    /// The agent's response text
    pub response: String,
    /// The session ID (can be used to resume this session later)
    pub session_id: String,
}

/// Run a one-shot prompt through ACP and return the response
///
/// This spawns the agent, initializes ACP, sends the prompt, collects the
/// response, and shuts down. Designed for Staged's single-request use case
/// (e.g., diff analysis).
///
/// Runs in a dedicated thread with its own LocalSet to handle !Send futures.
pub async fn run_acp_prompt(
    agent: &AcpAgent,
    working_dir: &Path,
    prompt: &str,
) -> Result<String, String> {
    let result = run_acp_prompt_with_session(agent, working_dir, prompt, None).await?;
    Ok(result.response)
}

/// Run a prompt through ACP with optional session resumption
///
/// If `session_id` is provided, attempts to load and resume that session.
/// Otherwise, creates a new session. Returns both the response and the
/// session ID for future resumption.
///
/// Sessions are persisted in Goose's SQLite database, so they survive
/// process restarts.
pub async fn run_acp_prompt_with_session(
    agent: &AcpAgent,
    working_dir: &Path,
    prompt: &str,
    session_id: Option<&str>,
) -> Result<AcpPromptResult, String> {
    let agent_path = agent.path().to_path_buf();
    let agent_name = agent.name().to_string();
    let agent_args: Vec<String> = agent.acp_args().iter().map(|s| s.to_string()).collect();
    let working_dir = working_dir.to_path_buf();
    let prompt = prompt.to_string();
    let session_id = session_id.map(|s| s.to_string());

    // Run the ACP session in a blocking task with its own runtime
    // This is needed because ACP uses !Send futures (LocalSet)
    tokio::task::spawn_blocking(move || {
        // Create a new runtime for this thread
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        // Run the ACP session on a LocalSet
        let local = tokio::task::LocalSet::new();
        local.block_on(&rt, async move {
            run_acp_session_inner(
                &agent_path,
                &agent_name,
                &agent_args,
                &working_dir,
                &prompt,
                session_id.as_deref(),
            )
            .await
        })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Internal function to run the ACP session (runs on LocalSet)
async fn run_acp_session_inner(
    agent_path: &Path,
    agent_name: &str,
    agent_args: &[String],
    working_dir: &Path,
    prompt: &str,
    existing_session_id: Option<&str>,
) -> Result<AcpPromptResult, String> {
    // Spawn the agent process with ACP mode
    let mut cmd = Command::new(agent_path);
    cmd.args(agent_args)
        .current_dir(working_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true); // Ensure child is killed if we exit early

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn {}: {}", agent_name, e))?;

    // Get stdin/stdout/stderr
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Failed to get stdin from agent process".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to get stdout from agent process".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to get stderr from agent process".to_string())?;

    // Spawn a task to read and log stderr
    let agent_name_for_stderr = agent_name.to_string();
    tokio::task::spawn_local(async move {
        use tokio::io::AsyncBufReadExt;
        let reader = tokio::io::BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            log::debug!("[ACP stderr {}] {}", agent_name_for_stderr, line);
        }
    });

    // Convert to futures-compatible async read/write
    let stdin_compat = stdin.compat_write();
    let stdout_compat = stdout.compat();

    // Create response collector
    let collector = Arc::new(ResponseCollector {
        accumulated_content: Mutex::new(String::new()),
    });

    // Create client handler
    let client = StagedAcpClient {
        collector: collector.clone(),
    };

    // Create the ACP connection
    let (connection, io_future) =
        ClientSideConnection::new(client, stdin_compat, stdout_compat, |fut| {
            tokio::task::spawn_local(fut);
        });

    // Spawn the IO task
    tokio::task::spawn_local(async move {
        if let Err(e) = io_future.await {
            log::error!("ACP IO error: {:?}", e);
        }
    });

    // Initialize the connection
    let client_info = Implementation::new("staged", env!("CARGO_PKG_VERSION"));
    let init_request = InitializeRequest::new(ProtocolVersion::LATEST).client_info(client_info);

    log::debug!("[ACP Request] initialize: {:?}", init_request);
    let init_response = connection
        .initialize(init_request)
        .await
        .map_err(|e| format!("Failed to initialize ACP connection: {:?}", e))?;
    log::debug!("[ACP Response] initialize: {:?}", init_response);

    if let Some(agent_info) = &init_response.agent_info {
        log::info!(
            "Connected to agent: {} v{}",
            agent_info.name,
            agent_info.version
        );
    }

    // Get or create session, track if this is a new session
    let (session_id, is_new_session): (SessionId, bool) =
        if let Some(existing_id) = existing_session_id {
            // Try to load existing session
            log::info!("Attempting to load session: {}", existing_id);
            let load_request =
                LoadSessionRequest::new(SessionId::new(existing_id), working_dir.to_path_buf());

            log::debug!("[ACP Request] load_session: {:?}", load_request);
            match connection.load_session(load_request).await {
                Ok(load_response) => {
                    log::debug!("[ACP Response] load_session: {:?}", load_response);
                    log::info!("Resumed session: {}", existing_id);
                    (SessionId::new(existing_id), false)
                }
                Err(e) => {
                    // Session not found or error - create a new one
                    log::debug!("[ACP Response] load_session error: {:?}", e);
                    log::warn!(
                        "Failed to load session {}: {:?}, creating new session",
                        existing_id,
                        e
                    );
                    let new_session_request = NewSessionRequest::new(working_dir.to_path_buf());
                    log::debug!("[ACP Request] new_session: {:?}", new_session_request);
                    let session_response = connection
                        .new_session(new_session_request)
                        .await
                        .map_err(|e| format!("Failed to create ACP session: {:?}", e))?;
                    log::debug!("[ACP Response] new_session: {:?}", session_response);
                    (session_response.session_id, true)
                }
            }
        } else {
            // Create new session
            let new_session_request = NewSessionRequest::new(working_dir.to_path_buf());
            log::debug!("[ACP Request] new_session: {:?}", new_session_request);
            let session_response = connection
                .new_session(new_session_request)
                .await
                .map_err(|e| format!("Failed to create ACP session: {:?}", e))?;
            log::debug!("[ACP Response] new_session: {:?}", session_response);
            log::info!("Created new session: {}", session_response.session_id.0);
            (session_response.session_id, true)
        };

    // Clear any accumulated content from loading session history
    // (load_session may replay old messages as AgentMessageChunk notifications)
    collector.accumulated_content.lock().await.clear();

    // For new sessions, prepend system context to guide the agent's behavior
    let full_prompt = if is_new_session {
        format!("{}{}", STAGED_SYSTEM_CONTEXT, prompt)
    } else {
        prompt.to_string()
    };

    // Send the prompt
    let prompt_request = PromptRequest::new(
        session_id.clone(),
        vec![AcpContentBlock::Text(TextContent::new(full_prompt))],
    );

    log::debug!("[ACP Request] prompt: {:?}", prompt_request);
    let prompt_response = connection
        .prompt(prompt_request)
        .await
        .map_err(|e| format!("Failed to send prompt: {:?}", e))?;
    log::debug!("[ACP Response] prompt: {:?}", prompt_response);

    // Clean up the child process
    let _ = child.kill().await;

    // Get the accumulated response
    let response = collector.accumulated_content.lock().await.clone();

    Ok(AcpPromptResult {
        response,
        session_id: session_id.0.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_acp_agent() {
        // This test just verifies the function doesn't panic
        // Actual availability depends on the system
        let _ = find_acp_agent();
    }
}
