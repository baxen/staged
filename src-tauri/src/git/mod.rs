mod cli;
mod commit;
mod diff;
mod files;
pub mod github;
mod refs;
mod types;

use std::path::Path;

pub use cli::GitError;
pub use commit::commit;
pub use diff::{get_file_diff, list_diff_files};
pub use files::{get_file_at_ref, search_files};
pub use github::{
    check_github_auth, fetch_pr, invalidate_cache as invalidate_pr_cache, list_pull_requests,
    sync_review_to_github, GitHubAuthStatus, GitHubSyncResult, PullRequest,
};
pub use refs::{get_repo_root, list_refs, merge_base, resolve_ref};
pub use types::*;

/// Get git status --short output
pub fn status_short(repo: &Path) -> Result<String, GitError> {
    cli::run(repo, &["status", "--short"])
}

/// Stage all changes (git add --all)
pub fn add_all(repo: &Path) -> Result<String, GitError> {
    cli::run(repo, &["add", "--all"])
}

/// Create a commit with a message (assumes files are already staged)
pub fn commit_with_message(repo: &Path, message: &str) -> Result<String, GitError> {
    cli::run(repo, &["commit", "-m", message])
}

/// Get the full SHA of HEAD
pub fn get_head_sha(repo: &Path) -> Result<String, GitError> {
    let sha = cli::run(repo, &["rev-parse", "HEAD"])?;
    Ok(sha.trim().to_string())
}
