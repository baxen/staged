use super::cli::{self, GitError};
use super::types::*;
use git2::{DiffOptions, Repository};
use std::cell::RefCell;
use std::path::Path;

/// A hunk from git diff (0-indexed line numbers)
#[derive(Debug, Clone, Copy)]
struct Hunk {
    /// Start line in old file (0-indexed)
    old_start: u32,
    /// Number of lines in old file
    old_lines: u32,
    /// Start line in new file (0-indexed)
    new_start: u32,
    /// Number of lines in new file
    new_lines: u32,
}

/// List files changed in a diff (for sidebar)
/// Uses `git diff --name-status` for speed - no content loading
pub fn list_diff_files(repo: &Path, spec: &DiffSpec) -> Result<Vec<FileDiffSummary>, GitError> {
    let mut args = vec!["diff", "--name-status", "-z"];

    // Build the diff range
    // For WorkingTree head, we diff against base (staged or HEAD)
    // For commit..commit, we use base..head
    let is_working_tree = matches!(spec.head, GitRef::WorkingTree);

    match (&spec.base, &spec.head) {
        (GitRef::Rev(base), GitRef::WorkingTree) => {
            args.push(base.as_str());
        }
        (GitRef::Rev(base), GitRef::Rev(head)) => {
            args.push(base.as_str());
            args.push(head.as_str());
        }
        (GitRef::WorkingTree, _) => {
            return Err(GitError::CommandFailed(
                "Cannot use working tree as base".to_string(),
            ));
        }
    }

    let output = cli::run(repo, &args)?;
    let mut results = parse_name_status(&output)?;

    // For working tree diffs, also include untracked files
    // git diff doesn't show untracked files, so we get them separately
    if is_working_tree {
        let untracked = list_untracked_files(repo)?;
        results.extend(untracked);
    }

    Ok(results)
}

/// Parse `git diff --name-status -z` output
/// Format: STATUS\0PATH\0 (or STATUS\0OLD\0NEW\0 for renames)
fn parse_name_status(output: &str) -> Result<Vec<FileDiffSummary>, GitError> {
    let mut results = Vec::new();
    let mut parts = output.split('\0').peekable();

    while let Some(status) = parts.next() {
        if status.is_empty() {
            continue;
        }

        let status_char = status.chars().next().unwrap_or(' ');

        match status_char {
            'A' => {
                // Added: just one path
                if let Some(path) = parts.next() {
                    results.push(FileDiffSummary {
                        before: None,
                        after: Some(path.into()),
                    });
                }
            }
            'D' => {
                // Deleted: just one path
                if let Some(path) = parts.next() {
                    results.push(FileDiffSummary {
                        before: Some(path.into()),
                        after: None,
                    });
                }
            }
            'M' | 'T' => {
                // Modified or Type changed: just one path
                if let Some(path) = parts.next() {
                    results.push(FileDiffSummary {
                        before: Some(path.into()),
                        after: Some(path.into()),
                    });
                }
            }
            'R' | 'C' => {
                // Renamed or Copied: two paths (old, new)
                // Status might include similarity percentage like R100
                if let (Some(old), Some(new)) = (parts.next(), parts.next()) {
                    results.push(FileDiffSummary {
                        before: Some(old.into()),
                        after: Some(new.into()),
                    });
                }
            }
            _ => {
                // Unknown status, skip the path
                parts.next();
            }
        }
    }

    Ok(results)
}

/// List untracked files in the working directory
/// Uses `git ls-files --others --exclude-standard` to get files not tracked by git
fn list_untracked_files(repo: &Path) -> Result<Vec<FileDiffSummary>, GitError> {
    let args = ["ls-files", "--others", "--exclude-standard", "-z"];
    let output = cli::run(repo, &args)?;

    let results = output
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(|path| FileDiffSummary {
            before: None,
            after: Some(path.into()),
        })
        .collect();

    Ok(results)
}

/// Get full diff content for a single file using libgit2.
/// This is reliable and battle-tested - we use git CLI only for list_diff_files
/// where fsmonitor support matters for performance.
pub fn get_file_diff(repo_path: &Path, spec: &DiffSpec, path: &Path) -> Result<FileDiff, GitError> {
    let repo = Repository::discover(repo_path).map_err(|e| GitError::NotARepo(e.to_string()))?;

    // Resolve trees
    let base_tree = resolve_to_tree(&repo, &spec.base)?;
    let head_tree = resolve_to_tree(&repo, &spec.head)?;
    let is_working_tree = matches!(spec.head, GitRef::WorkingTree);

    // Load file content
    let before = load_file_from_tree(&repo, base_tree.as_ref(), path)?;
    let after = if is_working_tree {
        load_file_from_workdir(&repo, path)?
    } else {
        load_file_from_tree(&repo, head_tree.as_ref(), path)?
    };

    // Get hunks via libgit2
    let hunks = get_hunks_libgit2(
        &repo,
        base_tree.as_ref(),
        head_tree.as_ref(),
        is_working_tree,
        path,
    )?;

    // Compute alignments from hunks
    let alignments = compute_alignments_from_hunks(&hunks, &before, &after);

    Ok(FileDiff {
        before,
        after,
        alignments,
    })
}

/// Resolve a GitRef to a tree (or None for working tree)
fn resolve_to_tree<'a>(
    repo: &'a Repository,
    git_ref: &GitRef,
) -> Result<Option<git2::Tree<'a>>, GitError> {
    match git_ref {
        GitRef::WorkingTree => Ok(None),
        GitRef::Rev(rev) => {
            let obj = repo
                .revparse_single(rev)
                .map_err(|e| GitError::CommandFailed(format!("Cannot resolve '{}': {}", rev, e)))?;
            let tree = obj.peel_to_tree().map_err(|e| {
                GitError::CommandFailed(format!("Cannot get tree for '{}': {}", rev, e))
            })?;
            Ok(Some(tree))
        }
    }
}

/// Load file content from a git tree
fn load_file_from_tree(
    repo: &Repository,
    tree: Option<&git2::Tree>,
    path: &Path,
) -> Result<Option<File>, GitError> {
    let tree = match tree {
        Some(t) => t,
        None => return Ok(None),
    };

    let entry = match tree.get_path(path) {
        Ok(e) => e,
        Err(_) => return Ok(None), // File doesn't exist in this tree
    };

    let obj = entry
        .to_object(repo)
        .map_err(|e| GitError::CommandFailed(format!("Cannot load object: {}", e)))?;

    let blob = match obj.as_blob() {
        Some(b) => b,
        None => return Ok(None), // Not a file (maybe a submodule)
    };

    let content = bytes_to_content(blob.content());

    Ok(Some(File {
        path: path.to_string_lossy().to_string(),
        content,
    }))
}

/// Load file content from the working directory
fn load_file_from_workdir(repo: &Repository, path: &Path) -> Result<Option<File>, GitError> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::CommandFailed("Bare repository".into()))?;
    let full_path = workdir.join(path);

    if !full_path.exists() {
        return Ok(None);
    }

    // Skip directories (e.g., submodules)
    if full_path.is_dir() {
        return Ok(None);
    }

    let bytes = std::fs::read(&full_path)
        .map_err(|e| GitError::CommandFailed(format!("Cannot read file: {}", e)))?;

    Ok(Some(File {
        path: path.to_string_lossy().to_string(),
        content: bytes_to_content(&bytes),
    }))
}

/// Convert raw bytes to FileContent, detecting binary
fn bytes_to_content(bytes: &[u8]) -> FileContent {
    // Check for binary: look for null bytes in first 8KB
    let check_len = bytes.len().min(8192);
    if bytes[..check_len].contains(&0) {
        return FileContent::Binary;
    }

    // Parse as UTF-8 (lossy for display)
    let text = String::from_utf8_lossy(bytes);
    let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
    FileContent::Text { lines }
}

/// Get hunks for a single file using libgit2
fn get_hunks_libgit2(
    repo: &Repository,
    base_tree: Option<&git2::Tree>,
    head_tree: Option<&git2::Tree>,
    is_working_tree: bool,
    path: &Path,
) -> Result<Vec<Hunk>, GitError> {
    let mut opts = DiffOptions::new();
    opts.context_lines(0); // No context, just the changes
    opts.pathspec(path);

    let diff = if is_working_tree {
        repo.diff_tree_to_workdir_with_index(base_tree, Some(&mut opts))
    } else {
        repo.diff_tree_to_tree(base_tree, head_tree, Some(&mut opts))
    }
    .map_err(|e| GitError::CommandFailed(format!("Failed to compute diff: {}", e)))?;

    // Collect hunks
    let hunks: RefCell<Vec<Hunk>> = RefCell::new(Vec::new());

    diff.foreach(
        &mut |_delta, _progress| true, // file callback
        None,                          // binary callback
        Some(&mut |_delta, hunk| {
            // Git uses 1-indexed line numbers, convert to 0-indexed
            let old_start = if hunk.old_start() == 0 {
                0
            } else {
                hunk.old_start() - 1
            };
            let new_start = if hunk.new_start() == 0 {
                0
            } else {
                hunk.new_start() - 1
            };

            hunks.borrow_mut().push(Hunk {
                old_start,
                old_lines: hunk.old_lines(),
                new_start,
                new_lines: hunk.new_lines(),
            });
            true
        }),
        None, // line callback
    )
    .map_err(|e| GitError::CommandFailed(format!("Failed to iterate diff: {}", e)))?;

    Ok(hunks.into_inner())
}

/// Compute alignments from git hunks.
/// This uses git's authoritative diff output rather than recomputing.
fn compute_alignments_from_hunks(
    hunks: &[Hunk],
    before: &Option<File>,
    after: &Option<File>,
) -> Vec<Alignment> {
    let before_len = match before {
        Some(File {
            content: FileContent::Text { lines },
            ..
        }) => lines.len() as u32,
        _ => 0,
    };
    let after_len = match after {
        Some(File {
            content: FileContent::Text { lines },
            ..
        }) => lines.len() as u32,
        _ => 0,
    };

    // Handle empty files
    if before_len == 0 && after_len == 0 {
        return vec![];
    }

    // If no hunks but files exist, it's either all added or all deleted
    if hunks.is_empty() {
        if before_len == 0 {
            // All added
            return vec![Alignment {
                before: Span::new(0, 0),
                after: Span::new(0, after_len),
                changed: true,
            }];
        } else if after_len == 0 {
            // All deleted
            return vec![Alignment {
                before: Span::new(0, before_len),
                after: Span::new(0, 0),
                changed: true,
            }];
        } else {
            // No changes (shouldn't happen for files in a diff, but handle gracefully)
            return vec![Alignment {
                before: Span::new(0, before_len),
                after: Span::new(0, after_len),
                changed: false,
            }];
        }
    }

    let mut alignments = Vec::new();
    let mut before_pos = 0u32;
    let mut after_pos = 0u32;

    for hunk in hunks {
        // Unchanged region before this hunk
        if before_pos < hunk.old_start || after_pos < hunk.new_start {
            // The gap should be the same size on both sides for unchanged content
            let before_gap = hunk.old_start - before_pos;
            let after_gap = hunk.new_start - after_pos;

            // They should match for truly unchanged content, but handle edge cases
            if before_gap > 0 || after_gap > 0 {
                alignments.push(Alignment {
                    before: Span::new(before_pos, hunk.old_start),
                    after: Span::new(after_pos, hunk.new_start),
                    changed: false,
                });
            }
        }

        // The hunk itself (changed region)
        let hunk_before_end = hunk.old_start + hunk.old_lines;
        let hunk_after_end = hunk.new_start + hunk.new_lines;

        alignments.push(Alignment {
            before: Span::new(hunk.old_start, hunk_before_end),
            after: Span::new(hunk.new_start, hunk_after_end),
            changed: true,
        });

        before_pos = hunk_before_end;
        after_pos = hunk_after_end;
    }

    // Unchanged region after the last hunk
    if before_pos < before_len || after_pos < after_len {
        alignments.push(Alignment {
            before: Span::new(before_pos, before_len),
            after: Span::new(after_pos, after_len),
            changed: false,
        });
    }

    alignments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_name_status_added() {
        let output = "A\0new_file.txt\0";
        let result = parse_name_status(output).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].is_added());
        assert_eq!(
            result[0].after.as_ref().unwrap().to_str(),
            Some("new_file.txt")
        );
    }

    #[test]
    fn test_parse_name_status_deleted() {
        let output = "D\0old_file.txt\0";
        let result = parse_name_status(output).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].is_deleted());
    }

    #[test]
    fn test_parse_name_status_modified() {
        let output = "M\0changed.txt\0";
        let result = parse_name_status(output).unwrap();
        assert_eq!(result.len(), 1);
        assert!(!result[0].is_added());
        assert!(!result[0].is_deleted());
        assert!(!result[0].is_renamed());
    }

    #[test]
    fn test_parse_name_status_renamed() {
        let output = "R100\0old_name.txt\0new_name.txt\0";
        let result = parse_name_status(output).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].is_renamed());
        assert_eq!(
            result[0].before.as_ref().unwrap().to_str(),
            Some("old_name.txt")
        );
        assert_eq!(
            result[0].after.as_ref().unwrap().to_str(),
            Some("new_name.txt")
        );
    }

    #[test]
    fn test_parse_name_status_multiple() {
        let output = "A\0added.txt\0M\0modified.txt\0D\0deleted.txt\0";
        let result = parse_name_status(output).unwrap();
        assert_eq!(result.len(), 3);
    }
}
