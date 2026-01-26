//! Agent Manager - orchestrates agent connections and sessions
//!
//! The manager handles:
//! - Session lifecycle (create, close)
//! - Message routing to the appropriate agent connection
//! - Event forwarding to the frontend

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, Mutex};

use crate::agent::acp_client::{spawn_agent, ActiveAcpConnection};
use crate::agent::registry::AgentRegistry;
use crate::agent::types::{AgentConfig, AgentError, AgentEvent, Message, SessionId, SessionInfo};

/// Commands sent to the connection manager (runs on LocalSet)
#[allow(clippy::large_enum_variant)]
pub enum ConnectionCommand {
    /// Initialize a new connection
    Initialize {
        session_id: SessionId,
        config: Box<AgentConfig>,
        working_dir: PathBuf,
        name: String,
        event_sender: mpsc::UnboundedSender<AgentEvent>,
        /// Returns Result<acp_session_id, error>
        result_sender: oneshot::Sender<Result<String, AgentError>>,
        /// Optional existing ACP session ID to load (for session resume)
        existing_acp_session_id: Option<String>,
    },
    /// Send a prompt - returns the assistant's response text
    Prompt {
        session_id: SessionId,
        message: String,
        result_sender: oneshot::Sender<Result<String, AgentError>>,
    },
    /// Close a connection
    #[allow(dead_code)]
    Close { session_id: SessionId },
}

/// In-memory session storage
struct SessionData {
    info: SessionInfo,
    messages: Vec<Message>,
}

/// Manages agent sessions and coordinates with the connection manager
pub struct AgentManager {
    registry: AgentRegistry,
    sessions: Arc<Mutex<HashMap<SessionId, SessionData>>>,
    command_sender: mpsc::UnboundedSender<ConnectionCommand>,
}

impl AgentManager {
    /// Create a new agent manager
    pub fn new() -> (Self, mpsc::UnboundedReceiver<ConnectionCommand>) {
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let manager = Self {
            registry: AgentRegistry::new(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            command_sender,
        };

        (manager, command_receiver)
    }

    /// Get the agent registry
    pub fn registry(&self) -> &AgentRegistry {
        &self.registry
    }

    /// Create a new session with the specified agent
    pub async fn create_session(
        &self,
        agent_id: &str,
        working_dir: PathBuf,
        name: String,
        event_sender: mpsc::UnboundedSender<AgentEvent>,
    ) -> Result<SessionInfo, AgentError> {
        log::info!("create_session called with agent_id: {}", agent_id);

        // Get the agent config
        let config = self
            .registry
            .get(agent_id)
            .ok_or_else(|| AgentError::new(format!("Unknown agent: {}", agent_id)))?
            .clone();

        log::info!(
            "Resolved config: id={}, command={}, args={:?}",
            config.id,
            config.command,
            config.args
        );

        // Generate a session ID
        let session_id = uuid::Uuid::new_v4().to_string();

        // Create a channel for the result
        let (result_sender, result_receiver) = oneshot::channel();

        // Send the initialize command
        self.command_sender
            .send(ConnectionCommand::Initialize {
                session_id: session_id.clone(),
                config: Box::new(config.clone()),
                working_dir: working_dir.clone(),
                name: name.clone(),
                event_sender,
                result_sender,
                existing_acp_session_id: None,
            })
            .map_err(|_| AgentError::new("Failed to send initialize command"))?;

        // Wait for the result
        let acp_session_id = result_receiver
            .await
            .map_err(|_| AgentError::new("Failed to receive initialize result"))??;

        // Create session info
        let session_info = SessionInfo {
            id: session_id.clone(),
            name,
            agent_id: config.id,
            working_dir,
            created_at: chrono::Utc::now().timestamp(),
            message_count: 0,
            acp_session_id: Some(acp_session_id),
        };

        // Store the session in memory
        let session_data = SessionData {
            info: session_info.clone(),
            messages: vec![],
        };

        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id, session_data);

        Ok(session_info)
    }

    /// Send a message to a session
    pub async fn send_message(
        &self,
        session_id: &SessionId,
        message: String,
    ) -> Result<(), AgentError> {
        // Store the user message
        {
            let mut sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.messages.push(Message::user(&message));
                session.info.message_count = session.messages.len();
            }
        }

        // Create a channel for the result
        let (result_sender, result_receiver) = oneshot::channel();

        // Send the prompt command
        self.command_sender
            .send(ConnectionCommand::Prompt {
                session_id: session_id.clone(),
                message,
                result_sender,
            })
            .map_err(|_| AgentError::new("Failed to send prompt command"))?;

        // Wait for the result
        let response_text = result_receiver
            .await
            .map_err(|_| AgentError::new("Failed to receive prompt result"))??;

        // Store the assistant message
        if !response_text.is_empty() {
            let mut sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.messages.push(Message::assistant(&response_text));
                session.info.message_count = session.messages.len();
            }
        }

        Ok(())
    }

    /// Get current session info
    pub async fn get_session(&self, session_id: &SessionId) -> Option<SessionInfo> {
        let sessions = self.sessions.lock().await;
        sessions.get(session_id).map(|s| s.info.clone())
    }

    /// Close a session
    #[allow(dead_code)]
    pub async fn close_session(&self, session_id: &SessionId) {
        {
            let mut sessions = self.sessions.lock().await;
            sessions.remove(session_id);
        }

        let _ = self.command_sender.send(ConnectionCommand::Close {
            session_id: session_id.clone(),
        });
    }
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new().0
    }
}

/// Connection manager that runs on a LocalSet
///
/// This handles all ACP connections which require !Send futures.
pub struct ConnectionManager {
    connections: Rc<RefCell<HashMap<SessionId, Rc<ActiveAcpConnection>>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Process a command
    pub async fn handle_command(&mut self, command: ConnectionCommand) {
        match command {
            ConnectionCommand::Initialize {
                session_id,
                config,
                working_dir,
                name,
                event_sender,
                result_sender,
                existing_acp_session_id,
            } => {
                let result = self
                    .initialize_connection(
                        session_id.clone(),
                        config,
                        working_dir,
                        name,
                        event_sender,
                        existing_acp_session_id,
                    )
                    .await;
                let _ = result_sender.send(result);
            }
            ConnectionCommand::Prompt {
                session_id,
                message,
                result_sender,
            } => {
                let connections = self.connections.clone();
                tokio::task::spawn_local(async move {
                    let result = Self::send_prompt_async(connections, &session_id, &message).await;
                    let _ = result_sender.send(result);
                });
            }
            ConnectionCommand::Close { session_id } => {
                self.close_connection(&session_id).await;
            }
        }
    }

    async fn initialize_connection(
        &mut self,
        session_id: SessionId,
        config: Box<AgentConfig>,
        working_dir: PathBuf,
        name: String,
        event_sender: mpsc::UnboundedSender<AgentEvent>,
        existing_acp_session_id: Option<String>,
    ) -> Result<String, AgentError> {
        // Spawn the agent process
        let (_session, handle) = spawn_agent(
            &config,
            session_id.clone(),
            working_dir,
            name,
            event_sender,
            existing_acp_session_id,
        )
        .await?;

        // Initialize the connection
        let (connection, acp_session_id) = handle.initialize().await?;

        // Store the active connection
        self.connections
            .borrow_mut()
            .insert(session_id, Rc::new(connection));

        Ok(acp_session_id)
    }

    async fn send_prompt_async(
        connections: Rc<RefCell<HashMap<SessionId, Rc<ActiveAcpConnection>>>>,
        session_id: &SessionId,
        message: &str,
    ) -> Result<String, AgentError> {
        let connection = {
            let conns = connections.borrow();
            conns
                .get(session_id)
                .cloned()
                .ok_or_else(|| AgentError::new("Connection not found"))?
        };

        connection.prompt(message).await
    }

    async fn close_connection(&mut self, session_id: &SessionId) {
        let connection = self.connections.borrow_mut().remove(session_id);
        if let Some(connection) = connection {
            if let Ok(conn) = Rc::try_unwrap(connection) {
                if let Err(e) = conn.shutdown().await {
                    log::error!("Failed to shutdown connection: {}", e);
                }
            }
        }
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
