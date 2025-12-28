//! Review storage for Staged.
//!
//! Manages review sessions (comments, reviewed files, edits) keyed by diff identifiers.
//! A diff is identified by `base..head` where base is a SHA and head is either a SHA
//! or "@" for the working tree.
//!
//! ## Storage
//!
//! Reviews are stored in a SQLite database in the app's data directory.
//! This is shared across all repositories since SHAs are globally unique.

mod store;
mod types;

pub use store::{ReviewStore, SqliteStore, StoreError};
pub use types::*;

use std::path::PathBuf;
use std::sync::OnceLock;

/// Global store instance - initialized lazily on first access.
/// Uses OnceLock to ensure thread-safe initialization.
static STORE: OnceLock<Result<SqliteStore, String>> = OnceLock::new();

/// Get the database path for the current platform.
pub fn db_path() -> Result<PathBuf, StoreError> {
    // Use standard app data directories
    let base = if cfg!(target_os = "macos") {
        dirs::data_dir().map(|p| p.join("com.staged.app"))
    } else {
        dirs::data_dir().map(|p| p.join("staged"))
    };

    base.map(|p| p.join("staged.db")).ok_or_else(|| StoreError {
        message: "Could not determine app data directory".to_string(),
    })
}

/// Get or initialize the global store.
pub fn get_store() -> Result<&'static SqliteStore, StoreError> {
    let result = STORE.get_or_init(|| db_path().and_then(SqliteStore::open).map_err(|e| e.message));

    match result {
        Ok(store) => Ok(store),
        Err(msg) => Err(StoreError {
            message: msg.clone(),
        }),
    }
}

/// Export a review as markdown for clipboard.
pub fn export_markdown(review: &Review) -> String {
    let mut md = String::new();

    // Group comments by file
    let mut comments_by_file: std::collections::HashMap<&str, Vec<&Comment>> =
        std::collections::HashMap::new();
    for comment in &review.comments {
        comments_by_file
            .entry(&comment.file_path)
            .or_default()
            .push(comment);
    }

    // Group edits by file
    let mut edits_by_file: std::collections::HashMap<&str, Vec<&Edit>> =
        std::collections::HashMap::new();
    for edit in &review.edits {
        edits_by_file.entry(&edit.file_path).or_default().push(edit);
    }

    // Collect all files
    let mut all_files: Vec<&str> = comments_by_file
        .keys()
        .chain(edits_by_file.keys())
        .copied()
        .collect();
    all_files.sort();
    all_files.dedup();

    for file in all_files {
        md.push_str(&format!("## {}\n\n", file));

        if let Some(comments) = comments_by_file.get(file) {
            for comment in comments {
                md.push_str(&format!(
                    "- **Range {}**: {}\n",
                    comment.range_index, comment.text
                ));
            }
            md.push('\n');
        }

        if let Some(edits) = edits_by_file.get(file) {
            for edit in edits {
                md.push_str("**Edit applied:**\n```diff\n");
                md.push_str(&edit.diff);
                if !edit.diff.ends_with('\n') {
                    md.push('\n');
                }
                md.push_str("```\n\n");
            }
        }
    }

    if md.is_empty() {
        md.push_str("No comments or edits.\n");
    }

    md
}
