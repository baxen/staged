//! Diff operations

use super::repo::find_repo;
use super::GitError;
use git2::{Diff, DiffOptions, Repository};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Represents a single line in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_type: String, // "context", "added", "removed"
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub content: String,
}

/// Represents a hunk (chunk) of changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// A row in the side-by-side view - either a line of code or a collapse indicator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DiffRow {
    /// A line of code
    Line(DiffLine),
    /// A collapse indicator showing N lines exist on the other side
    Collapse {
        /// Number of lines collapsed (on the other pane)
        count: u32,
        /// Starting line number on the other pane
        start_line: u32,
        /// Index into the other pane's rows where this collapse corresponds
        other_pane_index: usize,
    },
}

/// Represents the complete diff for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>, // For renames
    pub status: String,
    pub hunks: Vec<DiffHunk>,
    pub is_binary: bool,
    pub old_content: Vec<DiffRow>, // Rows for left pane (original)
    pub new_content: Vec<DiffRow>, // Rows for right pane (modified)
}

/// Get diff for a specific file
/// `staged` parameter determines whether to get staged (index vs HEAD) or unstaged (working tree vs index) diff
pub fn get_file_diff(
    repo_path: Option<&str>,
    file_path: &str,
    staged: bool,
) -> Result<FileDiff, GitError> {
    let repo = find_repo(repo_path)?;

    let mut diff_opts = DiffOptions::new();
    diff_opts.pathspec(file_path);
    diff_opts.context_lines(0); // We'll show full file, don't need context from git

    let diff = if staged {
        // Staged: compare HEAD to index
        let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
        repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut diff_opts))?
    } else {
        // Unstaged: compare index to working directory
        repo.diff_index_to_workdir(None, Some(&mut diff_opts))?
    };

    // Get full file contents for both sides
    let old_file_content = get_old_file_content(&repo, file_path, staged)?;
    let new_file_content = get_new_file_content(&repo, file_path, staged)?;

    parse_diff_for_file(&diff, file_path, &old_file_content, &new_file_content)
}

/// Get the "old" file content (what we're comparing from)
/// - For staged diffs: content from HEAD
/// - For unstaged diffs: content from index
fn get_old_file_content(
    repo: &Repository,
    file_path: &str,
    staged: bool,
) -> Result<Option<String>, GitError> {
    if staged {
        // Get from HEAD
        let head = match repo.head() {
            Ok(h) => h,
            Err(_) => return Ok(None), // No HEAD (initial commit)
        };
        let tree = head.peel_to_tree().map_err(|e| GitError {
            message: format!("Failed to get HEAD tree: {}", e),
        })?;
        let entry = match tree.get_path(std::path::Path::new(file_path)) {
            Ok(e) => e,
            Err(_) => return Ok(None), // File doesn't exist in HEAD (new file)
        };
        let blob = repo.find_blob(entry.id()).map_err(|e| GitError {
            message: format!("Failed to get blob: {}", e),
        })?;
        if blob.is_binary() {
            return Ok(None);
        }
        Ok(Some(String::from_utf8_lossy(blob.content()).into_owned()))
    } else {
        // Get from index
        let index = repo.index().map_err(|e| GitError {
            message: format!("Failed to get index: {}", e),
        })?;
        let entry = match index.get_path(std::path::Path::new(file_path), 0) {
            Some(e) => e,
            None => return Ok(None), // File not in index
        };
        let blob = repo.find_blob(entry.id).map_err(|e| GitError {
            message: format!("Failed to get blob: {}", e),
        })?;
        if blob.is_binary() {
            return Ok(None);
        }
        Ok(Some(String::from_utf8_lossy(blob.content()).into_owned()))
    }
}

/// Get the "new" file content (what we're comparing to)
/// - For staged diffs: content from index
/// - For unstaged diffs: content from working directory
fn get_new_file_content(
    repo: &Repository,
    file_path: &str,
    staged: bool,
) -> Result<Option<String>, GitError> {
    if staged {
        // Get from index
        let index = repo.index().map_err(|e| GitError {
            message: format!("Failed to get index: {}", e),
        })?;
        let entry = match index.get_path(std::path::Path::new(file_path), 0) {
            Some(e) => e,
            None => return Ok(None), // File deleted from index
        };
        let blob = repo.find_blob(entry.id).map_err(|e| GitError {
            message: format!("Failed to get blob: {}", e),
        })?;
        if blob.is_binary() {
            return Ok(None);
        }
        Ok(Some(String::from_utf8_lossy(blob.content()).into_owned()))
    } else {
        // Get from working directory
        let workdir = repo.workdir().ok_or_else(|| GitError {
            message: "Repository has no working directory".to_string(),
        })?;
        let full_path = workdir.join(file_path);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => Ok(Some(content)),
            Err(_) => Ok(None), // File deleted from working directory
        }
    }
}

/// Get diff for an untracked file (show entire file as added)
pub fn get_untracked_file_diff(
    repo_path: Option<&str>,
    file_path: &str,
) -> Result<FileDiff, GitError> {
    let repo = find_repo(repo_path)?;
    let workdir = repo.workdir().ok_or_else(|| GitError {
        message: "Repository has no working directory".to_string(),
    })?;

    let full_path = workdir.join(file_path);
    let content = std::fs::read_to_string(&full_path).map_err(|e| GitError {
        message: format!("Failed to read file: {}", e),
    })?;

    let lines: Vec<DiffLine> = content
        .lines()
        .enumerate()
        .map(|(i, line)| DiffLine {
            line_type: "added".to_string(),
            old_lineno: None,
            new_lineno: Some((i + 1) as u32),
            content: line.to_string(),
        })
        .collect();

    let line_count = lines.len();

    // For untracked files: left pane has collapse indicator, right pane has all lines
    let old_content = if line_count > 0 {
        vec![DiffRow::Collapse {
            count: line_count as u32,
            start_line: 1,
            other_pane_index: 0,
        }]
    } else {
        vec![]
    };

    let new_content: Vec<DiffRow> = lines.iter().cloned().map(DiffRow::Line).collect();

    Ok(FileDiff {
        path: file_path.to_string(),
        old_path: None,
        status: "untracked".to_string(),
        hunks: vec![DiffHunk {
            old_start: 0,
            old_lines: 0,
            new_start: 1,
            new_lines: line_count as u32,
            header: format!("@@ -0,0 +1,{} @@", line_count),
            lines,
        }],
        is_binary: false,
        old_content,
        new_content,
    })
}

/// Parse a git2 Diff object and extract information for a specific file
fn parse_diff_for_file(
    diff: &Diff,
    target_path: &str,
    old_file_content: &Option<String>,
    new_file_content: &Option<String>,
) -> Result<FileDiff, GitError> {
    use std::cell::RefCell;

    let hunks: RefCell<Vec<DiffHunk>> = RefCell::new(Vec::new());
    let is_binary: RefCell<bool> = RefCell::new(false);
    let file_status: RefCell<String> = RefCell::new("modified".to_string());
    let old_path: RefCell<Option<String>> = RefCell::new(None);
    let found_file: RefCell<bool> = RefCell::new(false);

    let current_hunk_lines: RefCell<Vec<DiffLine>> = RefCell::new(Vec::new());
    let current_hunk_header: RefCell<String> = RefCell::new(String::new());
    let current_hunk_old_start: RefCell<u32> = RefCell::new(0);
    let current_hunk_old_lines: RefCell<u32> = RefCell::new(0);
    let current_hunk_new_start: RefCell<u32> = RefCell::new(0);
    let current_hunk_new_lines: RefCell<u32> = RefCell::new(0);
    let in_target_file: RefCell<bool> = RefCell::new(false);

    diff.foreach(
        &mut |delta, _progress| {
            let new_file_path = delta.new_file().path().and_then(|p| p.to_str());
            let old_file_path = delta.old_file().path().and_then(|p| p.to_str());

            let is_target =
                new_file_path == Some(target_path) || old_file_path == Some(target_path);
            *in_target_file.borrow_mut() = is_target;

            if is_target {
                *found_file.borrow_mut() = true;
                *is_binary.borrow_mut() =
                    delta.new_file().is_binary() || delta.old_file().is_binary();

                *file_status.borrow_mut() = match delta.status() {
                    git2::Delta::Added => "added",
                    git2::Delta::Deleted => "deleted",
                    git2::Delta::Modified => "modified",
                    git2::Delta::Renamed => "renamed",
                    git2::Delta::Copied => "copied",
                    _ => "modified",
                }
                .to_string();

                if delta.status() == git2::Delta::Renamed {
                    *old_path.borrow_mut() = old_file_path.map(|s| s.to_string());
                }
            }
            true
        },
        None, // binary_cb
        Some(&mut |_delta, hunk| {
            if *in_target_file.borrow() {
                // Save previous hunk if exists
                let mut lines = current_hunk_lines.borrow_mut();
                if !lines.is_empty() {
                    hunks.borrow_mut().push(DiffHunk {
                        old_start: *current_hunk_old_start.borrow(),
                        old_lines: *current_hunk_old_lines.borrow(),
                        new_start: *current_hunk_new_start.borrow(),
                        new_lines: *current_hunk_new_lines.borrow(),
                        header: current_hunk_header.borrow().clone(),
                        lines: lines.clone(),
                    });
                    lines.clear();
                }

                *current_hunk_old_start.borrow_mut() = hunk.old_start();
                *current_hunk_old_lines.borrow_mut() = hunk.old_lines();
                *current_hunk_new_start.borrow_mut() = hunk.new_start();
                *current_hunk_new_lines.borrow_mut() = hunk.new_lines();
                *current_hunk_header.borrow_mut() =
                    String::from_utf8_lossy(hunk.header()).to_string();
            }
            true
        }),
        Some(&mut |_delta, _hunk, line| {
            if *in_target_file.borrow() {
                let line_type = match line.origin() {
                    '+' => "added",
                    '-' => "removed",
                    ' ' => "context",
                    _ => "context",
                }
                .to_string();

                let content = String::from_utf8_lossy(line.content())
                    .trim_end_matches('\n')
                    .trim_end_matches('\r')
                    .to_string();

                current_hunk_lines.borrow_mut().push(DiffLine {
                    line_type,
                    old_lineno: line.old_lineno(),
                    new_lineno: line.new_lineno(),
                    content,
                });
            }
            true
        }),
    )
    .map_err(|e| GitError {
        message: format!("Failed to parse diff: {}", e),
    })?;

    if !*found_file.borrow() {
        return Err(GitError {
            message: format!("File not found in diff: {}", target_path),
        });
    }

    if *is_binary.borrow() {
        return Ok(FileDiff {
            path: target_path.to_string(),
            old_path: old_path.into_inner(),
            status: file_status.into_inner(),
            hunks: vec![],
            is_binary: true,
            old_content: vec![],
            new_content: vec![],
        });
    }

    // Don't forget the last hunk
    let lines = current_hunk_lines.borrow();
    if !lines.is_empty() {
        hunks.borrow_mut().push(DiffHunk {
            old_start: *current_hunk_old_start.borrow(),
            old_lines: *current_hunk_old_lines.borrow(),
            new_start: *current_hunk_new_start.borrow(),
            new_lines: *current_hunk_new_lines.borrow(),
            header: current_hunk_header.borrow().clone(),
            lines: lines.clone(),
        });
    }
    drop(lines);

    let hunks = hunks.into_inner();

    // Build side-by-side content from full file contents and hunks
    let (old_content, new_content) =
        build_full_file_side_by_side(old_file_content, new_file_content, &hunks);

    Ok(FileDiff {
        path: target_path.to_string(),
        old_path: old_path.into_inner(),
        status: file_status.into_inner(),
        hunks,
        is_binary: false,
        old_content,
        new_content,
    })
}

/// Build side-by-side content from full file contents, using hunks to identify changes.
/// Shows the complete file with changed lines highlighted and collapse indicators.
fn build_full_file_side_by_side(
    old_file_content: &Option<String>,
    new_file_content: &Option<String>,
    hunks: &[DiffHunk],
) -> (Vec<DiffRow>, Vec<DiffRow>) {
    let old_lines: Vec<&str> = old_file_content
        .as_ref()
        .map(|s| s.lines().collect())
        .unwrap_or_default();
    let new_lines: Vec<&str> = new_file_content
        .as_ref()
        .map(|s| s.lines().collect())
        .unwrap_or_default();

    // Build sets of changed line numbers from hunks
    let mut old_removed: HashSet<u32> = HashSet::new();
    let mut new_added: HashSet<u32> = HashSet::new();

    for hunk in hunks {
        for line in &hunk.lines {
            match line.line_type.as_str() {
                "removed" => {
                    if let Some(lineno) = line.old_lineno {
                        old_removed.insert(lineno);
                    }
                }
                "added" => {
                    if let Some(lineno) = line.new_lineno {
                        new_added.insert(lineno);
                    }
                }
                _ => {}
            }
        }
    }

    let mut old_content: Vec<DiffRow> = Vec::new();
    let mut new_content: Vec<DiffRow> = Vec::new();

    let mut old_idx: usize = 0; // 0-indexed into old_lines
    let mut new_idx: usize = 0; // 0-indexed into new_lines

    // Process hunks in order, filling in unchanged lines between them
    for hunk in hunks {
        let hunk_old_start = hunk.old_start as usize;
        let hunk_new_start = hunk.new_start as usize;

        // Add unchanged lines before this hunk (context lines that aren't in any hunk)
        // These go to both panes simultaneously
        while old_idx + 1 < hunk_old_start && new_idx + 1 < hunk_new_start {
            let old_lineno = (old_idx + 1) as u32;
            let new_lineno = (new_idx + 1) as u32;

            let content = old_lines.get(old_idx).unwrap_or(&"").to_string();

            old_content.push(DiffRow::Line(DiffLine {
                line_type: "context".to_string(),
                old_lineno: Some(old_lineno),
                new_lineno: None,
                content: content.clone(),
            }));
            new_content.push(DiffRow::Line(DiffLine {
                line_type: "context".to_string(),
                old_lineno: None,
                new_lineno: Some(new_lineno),
                content,
            }));

            old_idx += 1;
            new_idx += 1;
        }

        // Process the hunk - collect consecutive changes for collapse indicators
        let mut pending_removed: Vec<DiffLine> = Vec::new();
        let mut pending_added: Vec<DiffLine> = Vec::new();

        for line in &hunk.lines {
            match line.line_type.as_str() {
                "context" => {
                    // Flush pending changes
                    flush_pending_changes(
                        &mut old_content,
                        &mut new_content,
                        &mut pending_removed,
                        &mut pending_added,
                    );

                    // Add context line to both sides
                    old_content.push(DiffRow::Line(DiffLine {
                        line_type: "context".to_string(),
                        old_lineno: line.old_lineno,
                        new_lineno: None,
                        content: line.content.clone(),
                    }));
                    new_content.push(DiffRow::Line(DiffLine {
                        line_type: "context".to_string(),
                        old_lineno: None,
                        new_lineno: line.new_lineno,
                        content: line.content.clone(),
                    }));

                    if let Some(ln) = line.old_lineno {
                        old_idx = ln as usize;
                    }
                    if let Some(ln) = line.new_lineno {
                        new_idx = ln as usize;
                    }
                }
                "removed" => {
                    // Flush added if we have them (handles interleaved)
                    if !pending_added.is_empty() {
                        flush_pending_changes(
                            &mut old_content,
                            &mut new_content,
                            &mut pending_removed,
                            &mut pending_added,
                        );
                    }
                    pending_removed.push(line.clone());
                    if let Some(ln) = line.old_lineno {
                        old_idx = ln as usize;
                    }
                }
                "added" => {
                    pending_added.push(line.clone());
                    if let Some(ln) = line.new_lineno {
                        new_idx = ln as usize;
                    }
                }
                _ => {}
            }
        }

        // Flush any remaining changes from this hunk
        flush_pending_changes(
            &mut old_content,
            &mut new_content,
            &mut pending_removed,
            &mut pending_added,
        );
    }

    // Add any remaining unchanged lines after the last hunk
    while old_idx < old_lines.len() && new_idx < new_lines.len() {
        let old_lineno = (old_idx + 1) as u32;
        let new_lineno = (new_idx + 1) as u32;

        let content = old_lines.get(old_idx).unwrap_or(&"").to_string();

        old_content.push(DiffRow::Line(DiffLine {
            line_type: "context".to_string(),
            old_lineno: Some(old_lineno),
            new_lineno: None,
            content: content.clone(),
        }));
        new_content.push(DiffRow::Line(DiffLine {
            line_type: "context".to_string(),
            old_lineno: None,
            new_lineno: Some(new_lineno),
            content,
        }));

        old_idx += 1;
        new_idx += 1;
    }

    // Handle case where one file is longer (shouldn't happen in normal diffs, but be safe)
    while old_idx < old_lines.len() {
        let old_lineno = (old_idx + 1) as u32;
        let content = old_lines.get(old_idx).unwrap_or(&"").to_string();
        old_content.push(DiffRow::Line(DiffLine {
            line_type: "context".to_string(),
            old_lineno: Some(old_lineno),
            new_lineno: None,
            content,
        }));
        old_idx += 1;
    }

    while new_idx < new_lines.len() {
        let new_lineno = (new_idx + 1) as u32;
        let content = new_lines.get(new_idx).unwrap_or(&"").to_string();
        new_content.push(DiffRow::Line(DiffLine {
            line_type: "context".to_string(),
            old_lineno: None,
            new_lineno: Some(new_lineno),
            content,
        }));
        new_idx += 1;
    }

    (old_content, new_content)
}

/// Flush pending removed/added lines, creating collapse indicators as needed
fn flush_pending_changes(
    old_content: &mut Vec<DiffRow>,
    new_content: &mut Vec<DiffRow>,
    pending_removed: &mut Vec<DiffLine>,
    pending_added: &mut Vec<DiffLine>,
) {
    if pending_removed.is_empty() && pending_added.is_empty() {
        return;
    }

    let removed_count = pending_removed.len();
    let added_count = pending_added.len();

    // Get starting line numbers for collapse indicators
    let removed_start_line = pending_removed
        .first()
        .and_then(|l| l.old_lineno)
        .unwrap_or(0);
    let added_start_line = pending_added
        .first()
        .and_then(|l| l.new_lineno)
        .unwrap_or(0);

    // Record positions before inserting
    let old_insert_index = old_content.len();
    let new_insert_index = new_content.len();

    // Add removed lines to old pane
    for line in pending_removed.drain(..) {
        old_content.push(DiffRow::Line(line));
    }

    // Add added lines to new pane
    for line in pending_added.drain(..) {
        new_content.push(DiffRow::Line(line));
    }

    // Add collapse indicator to new pane if there were removals
    if removed_count > 0 {
        new_content.insert(
            new_insert_index,
            DiffRow::Collapse {
                count: removed_count as u32,
                start_line: removed_start_line,
                other_pane_index: old_insert_index,
            },
        );
    }

    // Add collapse indicator to old pane if there were additions
    if added_count > 0 {
        let adjusted_new_index = if removed_count > 0 {
            new_insert_index + 1
        } else {
            new_insert_index
        };
        old_content.insert(
            old_insert_index + removed_count,
            DiffRow::Collapse {
                count: added_count as u32,
                start_line: added_start_line,
                other_pane_index: adjusted_new_index,
            },
        );
    }
}
