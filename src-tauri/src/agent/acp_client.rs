//! ACP Client - handles communication with agents via Agent Client Protocol
//!
//! This module spawns agent processes and communicates with them using ACP,
//! a JSON-RPC based protocol over stdio.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use agent_client_protocol::{
    Agent, ClientSideConnection, ContentBlock as AcpContentBlock, Implementation,
    InitializeRequest, LoadSessionRequest, NewSessionRequest, PermissionOptionId, PromptRequest,
    ProtocolVersion, RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    Result as AcpResult, SelectedPermissionOutcome, SessionNotification, SessionUpdate,
    TextContent, ToolCallStatus,
};
use async_trait::async_trait;
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::agent::types::{
    AgentConfig, AgentError, AgentEvent, ContentBlock, SessionId, SessionInfo, ToolResult,
};
use crate::cli_discovery;

/// Shared state for tracking the current response message ID
struct SharedState {
    current_message_id: Mutex<Option<String>>,
    /// Accumulated text content for the current response
    accumulated_content: Mutex<String>,
}

/// Spawn an agent process and prepare for ACP connection
pub async fn spawn_agent(
    config: &AgentConfig,
    session_id: SessionId,
    working_dir: PathBuf,
    name: String,
    event_sender: mpsc::UnboundedSender<AgentEvent>,
    existing_acp_session_id: Option<String>,
) -> Result<(SessionInfo, AcpConnectionHandle), AgentError> {
    // Substitute placeholders in args
    let resolved_args: Vec<String> = config
        .args
        .iter()
        .map(|arg| arg.replace("{session_id}", &session_id))
        .collect();

    // Resolve the command path using CLI discovery
    // This handles GUI apps not inheriting shell PATH
    let command_path = cli_discovery::find_command(&config.command)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| config.command.clone());

    log::info!(
        "Spawning agent '{}' with command: {} {:?}",
        config.id,
        command_path,
        resolved_args
    );

    // Spawn the agent process
    let mut cmd = Command::new(&command_path);
    cmd.args(&resolved_args)
        .current_dir(&working_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Set environment variables
    for (key, value) in &config.env {
        cmd.env(key, value);
    }

    let child = cmd.spawn().map_err(|e| {
        AgentError::new(format!(
            "Failed to spawn agent '{}': {}. Make sure '{}' is installed.",
            config.command, e, config.command
        ))
    })?;

    // Create session info
    let session_info = SessionInfo {
        id: session_id.clone(),
        name,
        agent_id: config.id.clone(),
        working_dir: working_dir.clone(),
        created_at: chrono::Utc::now().timestamp(),
        message_count: 0,
        acp_session_id: None, // Will be set after initialization
    };

    // Create shared state for message ID tracking
    let shared_state = Arc::new(SharedState {
        current_message_id: Mutex::new(None),
        accumulated_content: Mutex::new(String::new()),
    });

    // Create the connection handle
    let handle = AcpConnectionHandle {
        child,
        session_id,
        working_dir,
        event_sender,
        shared_state,
        existing_acp_session_id,
    };

    Ok((session_info, handle))
}

/// Handle to an ACP connection before initialization
pub struct AcpConnectionHandle {
    child: Child,
    session_id: SessionId,
    working_dir: PathBuf,
    event_sender: mpsc::UnboundedSender<AgentEvent>,
    shared_state: Arc<SharedState>,
    existing_acp_session_id: Option<String>,
}

impl AcpConnectionHandle {
    /// Initialize the ACP connection
    /// Returns the active connection and the ACP session ID (for persistence)
    pub async fn initialize(mut self) -> Result<(ActiveAcpConnection, String), AgentError> {
        // Get stdin/stdout from the child process
        let stdin = self
            .child
            .stdin
            .take()
            .ok_or_else(|| AgentError::new("Failed to get stdin from agent process"))?;
        let stdout = self
            .child
            .stdout
            .take()
            .ok_or_else(|| AgentError::new("Failed to get stdout from agent process"))?;

        // Convert to futures-compatible async read/write using compat
        let stdin_compat = stdin.compat_write();
        let stdout_compat = stdout.compat();

        // Create the client handler
        let client_handler = StagedAcpClient {
            session_id: self.session_id.clone(),
            event_sender: self.event_sender.clone(),
            shared_state: self.shared_state.clone(),
        };

        // Create the client-side connection
        let (connection, io_future) =
            ClientSideConnection::new(client_handler, stdin_compat, stdout_compat, |fut| {
                tokio::task::spawn_local(fut);
            });

        // Spawn the IO task on the local set
        tokio::task::spawn_local(async move {
            if let Err(e) = io_future.await {
                log::error!("ACP IO error: {:?}", e);
            }
        });

        // Build initialize request
        let client_info = Implementation::new("staged", env!("CARGO_PKG_VERSION"));
        let init_request = InitializeRequest::new(ProtocolVersion::LATEST).client_info(client_info);

        let init_response = connection.initialize(init_request).await.map_err(|e| {
            AgentError::new(format!("Failed to initialize ACP connection: {:?}", e))
        })?;

        // Log agent info
        if let Some(agent_info) = &init_response.agent_info {
            log::info!(
                "Connected to agent: {} v{}",
                agent_info.name,
                agent_info.version
            );
        } else {
            log::info!("Connected to agent (no agent info provided)");
        }

        // Either load an existing session or create a new one
        let acp_session_id = if let Some(existing_id) = self.existing_acp_session_id {
            log::info!("Loading existing ACP session: {}", existing_id);

            // Try to load the existing session
            let load_result = connection
                .load_session(LoadSessionRequest::new(
                    existing_id.clone(),
                    self.working_dir.clone(),
                ))
                .await;

            match load_result {
                Ok(_) => {
                    log::info!("Successfully loaded existing ACP session");
                    agent_client_protocol::SessionId::new(existing_id)
                }
                Err(e) => {
                    // Fall back to creating a new session if load fails
                    log::warn!(
                        "Failed to load existing ACP session, creating new one: {:?}",
                        e
                    );
                    let session_response = connection
                        .new_session(NewSessionRequest::new(self.working_dir.clone()))
                        .await
                        .map_err(|e| {
                            AgentError::new(format!("Failed to create ACP session: {:?}", e))
                        })?;
                    session_response.session_id
                }
            }
        } else {
            // Create a new session
            let session_response = connection
                .new_session(NewSessionRequest::new(self.working_dir.clone()))
                .await
                .map_err(|e| AgentError::new(format!("Failed to create ACP session: {:?}", e)))?;
            session_response.session_id
        };

        let acp_session_id_string = acp_session_id.to_string();

        Ok((
            ActiveAcpConnection {
                connection,
                child: self.child,
                session_id: self.session_id,
                acp_session_id,
                event_sender: self.event_sender,
                shared_state: self.shared_state,
            },
            acp_session_id_string,
        ))
    }
}

/// Client implementation for handling agent notifications
struct StagedAcpClient {
    session_id: SessionId,
    event_sender: mpsc::UnboundedSender<AgentEvent>,
    shared_state: Arc<SharedState>,
}

#[async_trait(?Send)]
impl agent_client_protocol::Client for StagedAcpClient {
    async fn request_permission(
        &self,
        args: RequestPermissionRequest,
    ) -> AcpResult<RequestPermissionResponse> {
        // Auto-approve by selecting the first option
        log::info!("Permission requested: {:?}", args);

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
        log::debug!("Session notification: {:?}", notification);

        match &notification.update {
            SessionUpdate::AgentMessageChunk(chunk) => {
                let message_id = {
                    let guard = self.shared_state.current_message_id.lock().await;
                    guard
                        .clone()
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
                };

                if let AcpContentBlock::Text(text) = &chunk.content {
                    // Accumulate content for persistence
                    {
                        let mut accumulated = self.shared_state.accumulated_content.lock().await;
                        accumulated.push_str(&text.text);
                    }

                    let _ = self.event_sender.send(AgentEvent::ContentChunk {
                        session_id: self.session_id.clone(),
                        message_id,
                        content: ContentBlock::Text {
                            text: text.text.clone(),
                        },
                    });
                }
            }
            SessionUpdate::ToolCall(tool_call) => {
                let _ = self.event_sender.send(AgentEvent::ToolCallStart {
                    session_id: self.session_id.clone(),
                    tool_call_id: tool_call.tool_call_id.to_string(),
                    tool_name: tool_call.title.clone(),
                    raw_input: tool_call.raw_input.clone(),
                });
            }
            SessionUpdate::ToolCallUpdate(tool_update) => {
                if let Some(status) = &tool_update.fields.status {
                    match status {
                        ToolCallStatus::Completed | ToolCallStatus::Failed => {
                            let extract_content_text = || -> Option<serde_json::Value> {
                                tool_update.fields.content.as_ref().and_then(|content_vec| {
                                    let texts: Vec<String> = content_vec
                                        .iter()
                                        .filter_map(|c| {
                                            use agent_client_protocol::ToolCallContent;
                                            match c {
                                                ToolCallContent::Content(content) => {
                                                    if let AcpContentBlock::Text(text) =
                                                        &content.content
                                                    {
                                                        Some(text.text.clone())
                                                    } else {
                                                        None
                                                    }
                                                }
                                                _ => None,
                                            }
                                        })
                                        .collect();

                                    if texts.is_empty() {
                                        None
                                    } else {
                                        Some(serde_json::Value::String(texts.join("\n")))
                                    }
                                })
                            };

                            let (status_str, value, error) =
                                if matches!(status, ToolCallStatus::Failed) {
                                    let error_msg = tool_update
                                        .fields
                                        .raw_output
                                        .as_ref()
                                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                                        .or_else(|| {
                                            tool_update
                                                .fields
                                                .raw_output
                                                .as_ref()
                                                .map(|v| v.to_string())
                                        })
                                        .or_else(|| {
                                            extract_content_text()
                                                .and_then(|v| v.as_str().map(|s| s.to_string()))
                                        });
                                    ("failed".to_string(), None, error_msg)
                                } else {
                                    let output = tool_update
                                        .fields
                                        .raw_output
                                        .clone()
                                        .or_else(extract_content_text);
                                    ("complete".to_string(), output, None)
                                };

                            let _ = self.event_sender.send(AgentEvent::ToolCallComplete {
                                session_id: self.session_id.clone(),
                                tool_call_id: tool_update.tool_call_id.to_string(),
                                result: ToolResult {
                                    status: status_str,
                                    value,
                                    error,
                                },
                                raw_input: tool_update.fields.raw_input.clone(),
                            });
                        }
                        _ => {}
                    }
                }
            }
            SessionUpdate::AgentThoughtChunk(chunk) => {
                if let AcpContentBlock::Text(text) = &chunk.content {
                    log::debug!("Agent thought: {}", text.text);
                }
            }
            _ => {
                log::debug!("Unhandled session update type: {:?}", notification.update);
            }
        }

        Ok(())
    }
}

/// An active ACP connection with an initialized session
pub struct ActiveAcpConnection {
    connection: ClientSideConnection,
    child: Child,
    session_id: SessionId,
    acp_session_id: agent_client_protocol::SessionId,
    event_sender: mpsc::UnboundedSender<AgentEvent>,
    shared_state: Arc<SharedState>,
}

impl ActiveAcpConnection {
    /// Send a prompt to the agent and return the complete response text
    pub async fn prompt(&self, message: &str) -> Result<String, AgentError> {
        let response_message_id = uuid::Uuid::new_v4().to_string();

        // Clear accumulated content and store the message ID
        {
            let mut accumulated = self.shared_state.accumulated_content.lock().await;
            accumulated.clear();
        }
        {
            let mut guard = self.shared_state.current_message_id.lock().await;
            *guard = Some(response_message_id.clone());
        }

        // Build the prompt request
        let prompt_request = PromptRequest::new(
            self.acp_session_id.clone(),
            vec![AcpContentBlock::Text(TextContent::new(message.to_string()))],
        );

        let response = self
            .connection
            .prompt(prompt_request)
            .await
            .map_err(|e| AgentError::new(format!("Failed to send prompt: {:?}", e)))?;

        // Get the accumulated content
        let accumulated_text = {
            let accumulated = self.shared_state.accumulated_content.lock().await;
            accumulated.clone()
        };

        // Clear the message ID
        {
            let mut guard = self.shared_state.current_message_id.lock().await;
            *guard = None;
        }

        // Send completion event
        let _ = self.event_sender.send(AgentEvent::Complete {
            session_id: self.session_id.clone(),
        });

        log::debug!("Prompt response: {:?}", response.stop_reason);

        Ok(accumulated_text)
    }

    /// Shutdown the connection
    pub async fn shutdown(mut self) -> Result<(), AgentError> {
        self.child
            .kill()
            .await
            .map_err(|e| AgentError::new(format!("Failed to kill agent process: {}", e)))?;
        Ok(())
    }
}
