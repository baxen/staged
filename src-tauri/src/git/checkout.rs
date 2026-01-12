use super::cli::{run, GitError};
use super::github::fetch_pr;
use std::path::Path;

/// Check if the working directory has uncommitted changes.
///
/// Returns true if there are any staged or unstaged changes that would be lost
/// by switching branches.
pub fn has_uncommitted_changes(repo: &Path) -> Result<bool, GitError> {
    let output = run(repo, &["status", "--porcelain"])?;
    // If status output is non-empty, there are changes
    Ok(!output.trim().is_empty())
}

/// Checkout a PR branch, creating a local tracking branch like "pr-123".
///
/// This function:
/// 1. Checks for uncommitted changes (returns error if dirty)
/// 2. Fetches the PR using GitHub's PR refs
/// 3. Creates or updates a local branch named "pr-{number}"
/// 4. Checks out that branch
///
/// Returns the name of the created/updated branch (e.g., "pr-123").
///
/// # Errors
///
/// - Returns error if there are uncommitted changes
/// - Returns error if PR fetch fails
/// - Returns error if branch creation/checkout fails
pub fn checkout_pr_branch(
    repo: &Path,
    pr_number: u64,
    base_ref: &str,
) -> Result<String, GitError> {
    // 1. Check for uncommitted changes
    if has_uncommitted_changes(repo)? {
        return Err(GitError::CommandFailed(
            "Cannot checkout PR: you have uncommitted changes. Commit or stash them first."
                .into(),
        ));
    }

    // 2. Fetch PR (reuses existing fetch_pr logic)
    let _diff_spec = fetch_pr(repo, base_ref, pr_number)?;

    // 3. Create local tracking branch "pr-{number}"
    let branch_name = format!("pr-{}", pr_number);
    let pr_ref = format!("refs/pull/{}/head", pr_number);

    // Check if branch exists
    let branch_exists = run(repo, &["rev-parse", "--verify", &format!("refs/heads/{}", branch_name)]).is_ok();

    if branch_exists {
        // Update existing branch to point to PR head
        run(repo, &["branch", "-f", &branch_name, &pr_ref])?;
    } else {
        // Create new branch
        run(repo, &["branch", &branch_name, &pr_ref])?;
    }

    // 4. Checkout the branch
    run(repo, &["checkout", &branch_name])?;

    log::info!(
        "Checked out PR #{} to branch '{}'",
        pr_number,
        branch_name
    );

    Ok(branch_name)
}
