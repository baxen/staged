//! Type definitions for projects, artifacts, and sessions.

use serde::{Deserialize, Serialize};

// =============================================================================
// Project
// =============================================================================

/// A goal-oriented collection of artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

// =============================================================================
// Artifact
// =============================================================================

/// The type of an artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    Markdown,
    Commit,
}

impl ArtifactType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ArtifactType::Markdown => "markdown",
            ArtifactType::Commit => "commit",
        }
    }
}

/// Type-specific data for an artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ArtifactData {
    Markdown {
        content: String,
    },
    Commit {
        repo: String,
        branch: String,
        #[serde(rename = "commitSha")]
        commit_sha: String,
    },
}

impl ArtifactData {
    pub fn artifact_type(&self) -> ArtifactType {
        match self {
            ArtifactData::Markdown { .. } => ArtifactType::Markdown,
            ArtifactData::Commit { .. } => ArtifactType::Commit,
        }
    }
}

/// The persistent output of AI work.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_artifact_id: Option<String>,
    /// The type-specific data (markdown content, commit info, etc.)
    pub data: ArtifactData,
}

impl Artifact {
    /// Create a new markdown artifact.
    pub fn new_markdown(
        project_id: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project_id.into(),
            title: title.into(),
            created_at: now.clone(),
            updated_at: now,
            parent_artifact_id: None,
            data: ArtifactData::Markdown {
                content: content.into(),
            },
        }
    }

    /// Get the artifact type.
    pub fn artifact_type(&self) -> ArtifactType {
        self.data.artifact_type()
    }
}

// =============================================================================
// Session
// =============================================================================

/// An ephemeral AI conversation that produced an artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub artifact_id: String,
    pub created_at: String,
    /// The conversation transcript (stored as JSON array of messages or raw text).
    pub transcript: String,
}

impl Session {
    pub fn new(artifact_id: impl Into<String>, transcript: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            artifact_id: artifact_id.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            transcript: transcript.into(),
        }
    }
}
