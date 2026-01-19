use super::cli::{self, GitError};
use std::path::Path;

/// Get the absolute path to the repository root.
pub fn get_repo_root(repo: &Path) -> Result<String, GitError> {
    let output = cli::run(repo, &["rev-parse", "--show-toplevel"])?;
    Ok(output.trim().to_string())
}

/// List refs (branches, tags, remotes) for autocomplete
pub fn list_refs(repo: &Path) -> Result<Vec<String>, GitError> {
    // Get all refs with a consistent format
    let output = cli::run(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads",
            "refs/remotes",
            "refs/tags",
        ],
    )?;

    let refs: Vec<String> = output.lines().map(|s| s.to_string()).collect();

    Ok(refs)
}

/// Compute the merge-base between two refs
pub fn merge_base(repo: &Path, ref1: &str, ref2: &str) -> Result<String, GitError> {
    let output = cli::run(repo, &["merge-base", ref1, ref2])?;
    Ok(output.trim().to_string())
}

/// Resolve a ref to its full SHA
pub fn resolve_ref(repo: &Path, reference: &str) -> Result<String, GitError> {
    let output = cli::run(repo, &["rev-parse", reference])?;
    Ok(output.trim().to_string())
}

/// Get the current branch name (or None if in detached HEAD state)
pub fn get_current_branch(repo: &Path) -> Result<Option<String>, GitError> {
    match cli::run(repo, &["symbolic-ref", "--short", "HEAD"]) {
        Ok(output) => Ok(Some(output.trim().to_string())),
        Err(GitError::CommandFailed(msg)) if msg.contains("not a symbolic ref") => {
            // Detached HEAD state
            Ok(None)
        }
        Err(e) => Err(e),
    }
}
