//! Staging operations (stage, unstage, discard)

use super::repo::find_repo;
use super::GitError;
use std::path::Path;

/// Stage a file (add to index)
pub fn stage_file(repo_path: Option<&str>, file_path: &str) -> Result<(), GitError> {
    let repo = find_repo(repo_path)?;
    let mut index = repo.index()?;

    // Check if file exists in working directory
    let workdir = repo.workdir().ok_or_else(|| GitError {
        message: "Repository has no working directory".to_string(),
    })?;

    let full_path = workdir.join(file_path);

    if full_path.exists() {
        // File exists - add it to index
        index.add_path(Path::new(file_path))?;
    } else {
        // File was deleted - remove from index
        index.remove_path(Path::new(file_path))?;
    }

    index.write()?;
    Ok(())
}

/// Unstage a file (remove from index, restore to HEAD state)
pub fn unstage_file(repo_path: Option<&str>, file_path: &str) -> Result<(), GitError> {
    let repo = find_repo(repo_path)?;

    // Get HEAD commit
    let head = repo.head().ok().and_then(|h| h.peel_to_commit().ok());

    match head {
        Some(commit) => {
            // Reset the file in index to match HEAD
            repo.reset_default(Some(&commit.into_object()), [file_path])?;
        }
        None => {
            // No HEAD (initial commit) - remove from index entirely
            let mut index = repo.index()?;
            index.remove_path(Path::new(file_path))?;
            index.write()?;
        }
    }

    Ok(())
}

/// Discard all changes to a file (restore to HEAD state).
///
/// This fully reverts a file to match HEAD:
/// - Unstages any staged changes (resets index to HEAD)
/// - Restores working directory to HEAD content
/// - Deletes the file if it doesn't exist in HEAD (newly added file)
///
/// This is the "nuclear option" - it removes ALL changes, both staged and unstaged.
pub fn discard_file(repo_path: Option<&str>, file_path: &str) -> Result<(), GitError> {
    let repo = find_repo(repo_path)?;
    let workdir = repo.workdir().ok_or_else(|| GitError {
        message: "Repository has no working directory".to_string(),
    })?;

    let full_path = workdir.join(file_path);

    // Get HEAD tree to find the original file state
    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());

    let head_entry = head_tree
        .as_ref()
        .and_then(|tree| tree.get_path(Path::new(file_path)).ok());

    // Step 1: Reset index to HEAD state for this file
    if let Some(commit) = repo.head().ok().and_then(|h| h.peel_to_commit().ok()) {
        // Use reset_default to restore index entry to HEAD
        let _ = repo.reset_default(Some(&commit.into_object()), [file_path]);
    } else {
        // No HEAD commit - remove from index if present
        let mut index = repo.index()?;
        let _ = index.remove_path(Path::new(file_path));
        index.write()?;
    }

    // Step 2: Restore working directory to HEAD state
    match head_entry {
        Some(entry) => {
            // File exists in HEAD - restore it
            let blob = repo.find_blob(entry.id()).map_err(|e| GitError {
                message: format!("Failed to get blob: {}", e),
            })?;
            let content = blob.content();

            // Create parent directories if needed
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| GitError {
                    message: format!("Failed to create directories: {}", e),
                })?;
            }

            std::fs::write(&full_path, content).map_err(|e| GitError {
                message: format!("Failed to write file: {}", e),
            })?;

            // Restore file permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = entry.filemode();
                if mode & 0o111 != 0 {
                    // File should be executable
                    let mut perms = std::fs::metadata(&full_path)
                        .map_err(|e| GitError {
                            message: format!("Failed to get metadata: {}", e),
                        })?
                        .permissions();
                    perms.set_mode(0o755);
                    std::fs::set_permissions(&full_path, perms).map_err(|e| GitError {
                        message: format!("Failed to set permissions: {}", e),
                    })?;
                }
            }
        }
        None => {
            // File doesn't exist in HEAD - delete it from working directory
            if full_path.exists() {
                if full_path.is_dir() {
                    std::fs::remove_dir_all(&full_path).map_err(|e| GitError {
                        message: format!("Failed to delete directory: {}", e),
                    })?;
                } else {
                    std::fs::remove_file(&full_path).map_err(|e| GitError {
                        message: format!("Failed to delete file: {}", e),
                    })?;
                }
            }
        }
    }

    Ok(())
}
