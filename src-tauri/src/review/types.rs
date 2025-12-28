//! Core types for review storage.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// A ref specification - either a SHA or "@" for working tree.
pub type RefSpec = String;

/// The special ref for the working tree (uncommitted changes).
pub const WORKING_TREE: &str = "@";

/// Identifies a diff by its two endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiffId {
    /// Base ref (SHA)
    pub base: RefSpec,
    /// Head ref (SHA or "@")
    pub head: RefSpec,
}

impl DiffId {
    pub fn new(base: impl Into<String>, head: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            head: head.into(),
        }
    }

    /// Create a stable ID for storage (hash of base..head).
    pub fn storage_id(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Check if this diff involves the working tree.
    pub fn is_working_tree(&self) -> bool {
        self.head == WORKING_TREE
    }
}

/// A review session for a specific diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    /// The diff being reviewed
    pub id: DiffId,
    /// File paths marked as reviewed
    pub reviewed: HashSet<String>,
    /// Comments attached to the diff
    pub comments: Vec<Comment>,
    /// Edits made during review
    pub edits: Vec<Edit>,
    /// When the review was created
    pub created_at: DateTime<Utc>,
    /// When the review was last updated
    pub updated_at: DateTime<Utc>,
}

impl Review {
    /// Create a new empty review for a diff.
    pub fn new(id: DiffId) -> Self {
        let now = Utc::now();
        Self {
            id,
            reviewed: HashSet::new(),
            comments: Vec::new(),
            edits: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// A comment attached to a range in the diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    /// Unique identifier
    pub id: Uuid,
    /// File the comment is on
    pub file_path: String,
    /// Index into the diff's ranges array
    pub range_index: usize,
    /// Comment text
    pub text: String,
    /// When the comment was created
    pub created_at: DateTime<Utc>,
}

impl Comment {
    pub fn new(file_path: impl Into<String>, range_index: usize, text: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            file_path: file_path.into(),
            range_index,
            text: text.into(),
            created_at: Utc::now(),
        }
    }
}

/// A recorded edit - stored as a diff for potential GitHub suggestion export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edit {
    /// Unique identifier
    pub id: Uuid,
    /// File the edit was made to
    pub file_path: String,
    /// Unified diff format of the change
    pub diff: String,
    /// When the edit was made
    pub created_at: DateTime<Utc>,
}

impl Edit {
    pub fn new(file_path: impl Into<String>, diff: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            file_path: file_path.into(),
            diff: diff.into(),
            created_at: Utc::now(),
        }
    }
}

/// Input for creating a new comment (from frontend).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewComment {
    pub file_path: String,
    pub range_index: usize,
    pub text: String,
}

/// Input for recording a new edit (from frontend).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewEdit {
    pub file_path: String,
    pub diff: String,
}
