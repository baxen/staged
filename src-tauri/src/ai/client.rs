//! ACP Client - handles communication with AI agents via Agent Client Protocol
//!
//! This module spawns agent processes and communicates with them using ACP,
//! a JSON-RPC based protocol over stdio. Supports both one-shot requests
//! (for diff analysis) and persistent sessions (for interactive chat).
//!
//! For streaming sessions, emits Tauri events with SDK types directly:
//! - "session-update": SessionNotification from the SDK
//! - "session-complete": Custom event with finalized transcript

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use agent_client_protocol::{
    Agent, ClientSideConnection, ContentBlock as AcpContentBlock, Implementation,
    InitializeRequest, LoadSessionRequest, NewSessionRequest, PermissionOptionId, PromptRequest,
    ProtocolVersion, RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    Result as AcpResult, SelectedPermissionOutcome, SessionId, SessionNotification, SessionUpdate,
    TextContent, ToolCall, ToolCallUpdate,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tauri::Emitter;
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

// =============================================================================
// Finalized Message Types (for database storage)
// =============================================================================

/// Summary of a tool call for storage (not streaming details)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_preview: Option<String>,
}

/// Finalized message for database storage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "camelCase")]
pub enum FinalizedMessage {
    User {
        content: String,
    },
    Assistant {
        content: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "toolCalls")]
        tool_calls: Vec<ToolCallSummary>,
    },
}

/// Event emitted when session completes
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCompleteEvent {
    pub session_id: String,
    pub transcript: Vec<FinalizedMessage>,
}

/// Event emitted when session errors
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionErrorEvent {
    pub session_id: String,
    pub error: String,
}

// =============================================================================
// Streaming Client Implementation
// =============================================================================

/// Internal state for tracking a tool call during streaming
#[derive(Debug, Clone)]
struct ToolCallState {
    id: String,
    title: String,
    status: String,
    locations: Vec<String>,
    result_preview: Option<String>,
}

impl From<&ToolCall> for ToolCallState {
    fn from(tc: &ToolCall) -> Self {
        Self {
            id: tc.tool_call_id.0.to_string(),
            title: tc.title.clone(),
            status: format!("{:?}", tc.status).to_lowercase(),
            locations: tc
                .locations
                .iter()
                .map(|l| l.path.display().to_string())
                .collect(),
            result_preview: None,
        }
    }
}

impl From<ToolCallState> for ToolCallSummary {
    fn from(state: ToolCallState) -> Self {
        Self {
            id: state.id,
            title: state.title,
            status: state.status,
            locations: state.locations,
            result_preview: state.result_preview,
        }
    }
}

/// Client implementation for handling agent notifications with streaming support
struct StreamingAcpClient {
    /// Tauri app handle for emitting events (None for non-streaming mode)
    app_handle: Option<tauri::AppHandle>,
    /// Session ID for event correlation
    session_id: Mutex<String>,
    /// Accumulated assistant message text
    current_message: Mutex<String>,
    /// Active tool calls by ID
    tool_calls: Mutex<HashMap<String, ToolCallState>>,
}

impl StreamingAcpClient {
    fn new(app_handle: Option<tauri::AppHandle>) -> Self {
        Self {
            app_handle,
            session_id: Mutex::new(String::new()),
            current_message: Mutex::new(String::new()),
            tool_calls: Mutex::new(HashMap::new()),
        }
    }

    async fn set_session_id(&self, id: &str) {
        *self.session_id.lock().await = id.to_string();
    }

    /// Emit a session update event to the frontend
    fn emit_update(&self, notification: &SessionNotification) {
        if let Some(ref app_handle) = self.app_handle {
            if let Err(e) = app_handle.emit("session-update", notification) {
                log::warn!("Failed to emit session-update event: {}", e);
            }
        }
    }

    /// Finalize the current message and tool calls into a transcript
    async fn finalize(&self) -> Vec<FinalizedMessage> {
        let content = std::mem::take(&mut *self.current_message.lock().await);
        let tool_calls: Vec<ToolCallSummary> = std::mem::take(&mut *self.tool_calls.lock().await)
            .into_values()
            .map(Into::into)
            .collect();

        let mut messages = Vec::new();

        if !content.is_empty() || !tool_calls.is_empty() {
            messages.push(FinalizedMessage::Assistant {
                content,
                tool_calls,
            });
        }

        messages
    }

    /// Get the accumulated response text (for non-streaming callers)
    async fn get_response(&self) -> String {
        self.current_message.lock().await.clone()
    }
}

#[async_trait(?Send)]
impl agent_client_protocol::Client for StreamingAcpClient {
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
        // 1. Emit the raw SDK notification to frontend (if streaming)
        self.emit_update(&notification);

        // 2. Update internal state for finalization
        match &notification.update {
            SessionUpdate::AgentMessageChunk(chunk) => {
                if let AcpContentBlock::Text(text) = &chunk.content {
                    self.current_message.lock().await.push_str(&text.text);
                }
            }
            SessionUpdate::ToolCall(tool_call) => {
                let state = ToolCallState::from(tool_call);
                self.tool_calls.lock().await.insert(state.id.clone(), state);
            }
            SessionUpdate::ToolCallUpdate(update) => {
                self.update_tool_call(update).await;
            }
            _ => {
                log::debug!("Ignoring session update: {:?}", notification.update);
            }
        }

        Ok(())
    }
}

impl StreamingAcpClient {
    async fn update_tool_call(&self, update: &ToolCallUpdate) {
        let mut tool_calls = self.tool_calls.lock().await;
        let id = update.tool_call_id.0.to_string();
        if let Some(tc) = tool_calls.get_mut(&id) {
            if let Some(ref status) = update.fields.status {
                tc.status = format!("{:?}", status).to_lowercase();
            }
            if let Some(ref title) = update.fields.title {
                tc.title = title.clone();
            }
            // Extract preview from content if available
            if let Some(ref content) = &update.fields.content {
                tc.result_preview = extract_content_preview(content);
            }
        }
    }
}

/// Extract a preview string from tool call content
fn extract_content_preview(content: &[agent_client_protocol::ToolCallContent]) -> Option<String> {
    for item in content {
        match item {
            agent_client_protocol::ToolCallContent::Content(c) => {
                if let AcpContentBlock::Text(text) = &c.content {
                    let preview: String = text.text.chars().take(200).collect();
                    return Some(if text.text.len() > 200 {
                        format!("{}...", preview)
                    } else {
                        preview
                    });
                }
            }
            agent_client_protocol::ToolCallContent::Diff(d) => {
                // Show a preview of the diff (old_text -> new_text)
                let preview = format!(
                    "{}{}",
                    d.path.display(),
                    if d.old_text.is_some() {
                        " (modified)"
                    } else {
                        " (new)"
                    }
                );
                return Some(preview);
            }
            agent_client_protocol::ToolCallContent::Terminal(t) => {
                return Some(format!("Terminal: {}", t.terminal_id.0));
            }
            _ => {}
        }
    }
    None
}

// =============================================================================
// Public API
// =============================================================================

/// Result of running an ACP prompt with session support
pub struct AcpPromptResult {
    /// The agent's response text
    pub response: String,
    /// The session ID (can be used to resume this session later)
    pub session_id: String,
    /// Finalized transcript for storage
    pub transcript: Vec<FinalizedMessage>,
}

/// Run a one-shot prompt through ACP and return the response (no streaming)
///
/// This spawns the agent, initializes ACP, sends the prompt, collects the
/// response, and shuts down. Designed for Staged's single-request use case
/// (e.g., diff analysis).
pub async fn run_acp_prompt(
    agent: &AcpAgent,
    working_dir: &Path,
    prompt: &str,
) -> Result<String, String> {
    let result = run_acp_prompt_internal(agent, working_dir, prompt, None, None).await?;
    Ok(result.response)
}

/// Run a prompt through ACP with optional session resumption (no streaming)
///
/// If `session_id` is provided, attempts to load and resume that session.
/// Otherwise, creates a new session. Returns both the response and the
/// session ID for future resumption.
pub async fn run_acp_prompt_with_session(
    agent: &AcpAgent,
    working_dir: &Path,
    prompt: &str,
    session_id: Option<&str>,
) -> Result<AcpPromptResult, String> {
    run_acp_prompt_internal(agent, working_dir, prompt, session_id, None).await
}

/// Run a prompt through ACP with streaming events emitted to frontend
///
/// Emits "session-update" events with SessionNotification payloads during execution.
/// Emits "session-complete" event with finalized transcript when done.
pub async fn run_acp_prompt_streaming(
    agent: &AcpAgent,
    working_dir: &Path,
    prompt: &str,
    session_id: Option<&str>,
    app_handle: tauri::AppHandle,
) -> Result<AcpPromptResult, String> {
    run_acp_prompt_internal(agent, working_dir, prompt, session_id, Some(app_handle)).await
}

/// Internal implementation that handles both streaming and non-streaming modes
async fn run_acp_prompt_internal(
    agent: &AcpAgent,
    working_dir: &Path,
    prompt: &str,
    session_id: Option<&str>,
    app_handle: Option<tauri::AppHandle>,
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
                app_handle,
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
    app_handle: Option<tauri::AppHandle>,
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

    // Get stdin/stdout
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Failed to get stdin from agent process".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to get stdout from agent process".to_string())?;

    // Convert to futures-compatible async read/write
    let stdin_compat = stdin.compat_write();
    let stdout_compat = stdout.compat();

    // Create streaming client
    let client = Arc::new(StreamingAcpClient::new(app_handle.clone()));
    let client_for_connection = Arc::clone(&client);

    // Create the ACP connection
    let (connection, io_future) =
        ClientSideConnection::new(client_for_connection, stdin_compat, stdout_compat, |fut| {
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

    let init_response = connection
        .initialize(init_request)
        .await
        .map_err(|e| format!("Failed to initialize ACP connection: {:?}", e))?;

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

            match connection.load_session(load_request).await {
                Ok(_) => {
                    log::info!("Resumed session: {}", existing_id);
                    (SessionId::new(existing_id), false)
                }
                Err(e) => {
                    // Session not found or error - create a new one
                    log::warn!(
                        "Failed to load session {}: {:?}, creating new session",
                        existing_id,
                        e
                    );
                    let session_response = connection
                        .new_session(NewSessionRequest::new(working_dir.to_path_buf()))
                        .await
                        .map_err(|e| format!("Failed to create ACP session: {:?}", e))?;
                    (session_response.session_id, true)
                }
            }
        } else {
            // Create new session
            let session_response = connection
                .new_session(NewSessionRequest::new(working_dir.to_path_buf()))
                .await
                .map_err(|e| format!("Failed to create ACP session: {:?}", e))?;
            log::info!("Created new session: {}", session_response.session_id.0);
            (session_response.session_id, true)
        };

    // Set session ID on client for event correlation
    client.set_session_id(&session_id.0).await;

    // Clear any accumulated content from loading session history
    // (load_session may replay old messages as AgentMessageChunk notifications)
    *client.current_message.lock().await = String::new();
    client.tool_calls.lock().await.clear();

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

    let prompt_result = connection.prompt(prompt_request).await;

    // Clean up the child process
    let _ = child.kill().await;

    // Handle result
    let session_id_str = session_id.0.to_string();

    match prompt_result {
        Ok(_) => {
            let response = client.get_response().await;
            let transcript = client.finalize().await;

            // Emit completion event if streaming
            if let Some(ref app_handle) = app_handle {
                let complete_event = SessionCompleteEvent {
                    session_id: session_id_str.clone(),
                    transcript: transcript.clone(),
                };
                if let Err(e) = app_handle.emit("session-complete", &complete_event) {
                    log::warn!("Failed to emit session-complete event: {}", e);
                }
            }

            Ok(AcpPromptResult {
                response,
                session_id: session_id_str,
                transcript,
            })
        }
        Err(e) => {
            // Emit error event if streaming
            if let Some(ref app_handle) = app_handle {
                let error_event = SessionErrorEvent {
                    session_id: session_id_str,
                    error: format!("{:?}", e),
                };
                if let Err(emit_err) = app_handle.emit("session-error", &error_event) {
                    log::warn!("Failed to emit session-error event: {}", emit_err);
                }
            }
            Err(format!("Failed to send prompt: {:?}", e))
        }
    }
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
