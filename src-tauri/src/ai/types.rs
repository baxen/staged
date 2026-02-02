//! Types for AI interactions and session management.

use serde::{Deserialize, Serialize};

/// Analysis result for a changeset (used by smart diff).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangesetAnalysis {
    pub summary: String,
    pub key_changes: Vec<String>,
    pub concerns: Vec<String>,
    pub file_annotations: std::collections::HashMap<String, Vec<FileAnnotation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnnotation {
    pub line: usize,
    pub message: String,
    #[serde(default)]
    pub severity: AnnotationSeverity,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationSeverity {
    #[default]
    Info,
    Warning,
    Error,
}

// =============================================================================
// Session Message Types
// =============================================================================

/// A message in an AI session transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionMessage {
    /// User's prompt/message
    User { content: String },
    /// Agent's response text (streamed in chunks, stored complete)
    Assistant { content: String },
    /// Agent's internal reasoning/thinking
    Thought { content: String },
    /// Tool call initiated by the agent
    ToolCall {
        id: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
    },
    /// Tool call result/update
    ToolResult {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
    },
}

impl SessionMessage {
    pub fn user(content: impl Into<String>) -> Self {
        SessionMessage::User {
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        SessionMessage::Assistant {
            content: content.into(),
        }
    }

    pub fn thought(content: impl Into<String>) -> Self {
        SessionMessage::Thought {
            content: content.into(),
        }
    }

    pub fn tool_call(id: impl Into<String>, name: impl Into<String>) -> Self {
        SessionMessage::ToolCall {
            id: id.into(),
            name: name.into(),
            status: None,
        }
    }

    pub fn tool_result(id: impl Into<String>) -> Self {
        SessionMessage::ToolResult {
            id: id.into(),
            content: None,
            status: None,
        }
    }
}

/// Event emitted during session streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStreamEvent {
    /// The artifact ID this session belongs to
    pub artifact_id: String,
    /// The new message or update
    pub message: SessionMessage,
}
