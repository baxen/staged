//! Core types for the agent system
//!
//! These types are shared between the Rust backend and TypeScript frontend.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================
// Session Types
// ============================================================================

/// Unique identifier for a session
pub type SessionId = String;

/// Information about a session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: SessionId,
    pub name: String,
    pub agent_id: String,
    pub working_dir: PathBuf,
    pub created_at: i64,
    pub message_count: usize,
    /// The ACP/goose session ID (for session resume)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acp_session_id: Option<String>,
}

/// A stored session with messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSession {
    pub info: SessionInfo,
    pub messages: Vec<Message>,
}

// ============================================================================
// Agent Configuration
// ============================================================================

/// Configuration for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    /// Unique identifier for the agent
    pub id: String,
    /// Display name
    pub name: String,
    /// Description of the agent
    pub description: String,
    /// Command to run the agent
    pub command: String,
    /// Arguments to pass to the command
    /// Supports placeholders: {session_id} is replaced at runtime
    pub args: Vec<String>,
    /// Environment variables to set
    pub env: Vec<(String, String)>,
    /// Icon identifier
    pub icon: Option<String>,
    /// Whether this agent is enabled
    pub enabled: bool,
    /// Environment variable name for API key (if required)
    /// e.g., "ANTHROPIC_API_KEY" for Claude Code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env_var: Option<String>,
    /// Argument to use for version/availability check (defaults to "--version")
    /// Some CLIs use "--help" or "version" instead
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_check_arg: Option<String>,
}

/// Information about an agent's availability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAvailability {
    pub id: String,
    pub available: bool,
    pub path: Option<String>,
}

// ============================================================================
// Message Types
// ============================================================================

/// Role of a message sender
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub created: i64,
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MessageMetadata>,
}

impl Message {
    /// Create a new user message
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::User,
            created: chrono::Utc::now().timestamp(),
            content: vec![ContentBlock::Text { text: text.into() }],
            metadata: Some(MessageMetadata::default()),
        }
    }

    /// Create a new assistant message
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            created: chrono::Utc::now().timestamp(),
            content: vec![ContentBlock::Text { text: text.into() }],
            metadata: Some(MessageMetadata::default()),
        }
    }
}

fn default_true() -> bool {
    true
}

/// Content block within a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ContentBlock {
    /// Text content
    Text { text: String },
    /// Tool request (agent wants to call a tool)
    ToolRequest {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    /// Tool response (result of a tool call)
    ToolResponse { id: String, result: ToolResult },
}

/// Result of a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Metadata for a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    #[serde(default = "default_true")]
    pub user_visible: bool,
    #[serde(default = "default_true")]
    pub agent_visible: bool,
}

impl Default for MessageMetadata {
    fn default() -> Self {
        Self {
            user_visible: true,
            agent_visible: true,
        }
    }
}

// ============================================================================
// Event Types
// ============================================================================

/// Events emitted during agent interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// A complete message
    Message {
        session_id: SessionId,
        message: Message,
    },
    /// A chunk of content (for streaming)
    ContentChunk {
        session_id: SessionId,
        message_id: String,
        content: ContentBlock,
    },
    /// Tool call started
    ToolCallStart {
        session_id: SessionId,
        tool_call_id: String,
        tool_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        raw_input: Option<serde_json::Value>,
    },
    /// Tool call completed
    ToolCallComplete {
        session_id: SessionId,
        tool_call_id: String,
        result: ToolResult,
        /// Raw input if it wasn't sent in the start event
        #[serde(skip_serializing_if = "Option::is_none")]
        raw_input: Option<serde_json::Value>,
    },
    /// Agent finished responding
    Complete { session_id: SessionId },
    /// Error occurred
    Error {
        session_id: SessionId,
        error: String,
    },
}

// ============================================================================
// Error Types
// ============================================================================

/// Error type for agent operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentError {
    pub message: String,
}

impl AgentError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AgentError {}

impl From<String> for AgentError {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for AgentError {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}
