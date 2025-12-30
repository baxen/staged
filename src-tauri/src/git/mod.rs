//! Git operations for Staged.
//!
//! Pure git2 operations with no Tauri dependency.
//! All functions are stateless - they discover the repo fresh each call.
//!
//! ## Module Structure
//! - `commit`: Create and amend commits
//! - `provider`: Status fetching with git2/CLI fallback
//! - `repo`: Repository discovery utilities
//! - `staging`: Stage, unstage, and discard operations
//! - `status`: Working tree and index status

mod commit;
pub mod provider;
mod repo;
mod staging;
mod status;

use serde::{Deserialize, Serialize};

// Re-export public types
pub use commit::CommitResult;
pub use provider::AdaptiveProvider;
pub use status::GitStatus;

// Re-export public functions
pub use commit::{amend_commit, create_commit, get_last_commit_message};
pub use staging::{discard_file, stage_file, unstage_file};
pub use status::get_status;

/// Common error type for git operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitError {
    pub message: String,
}

impl From<git2::Error> for GitError {
    fn from(err: git2::Error) -> Self {
        GitError {
            message: err.message().to_string(),
        }
    }
}
