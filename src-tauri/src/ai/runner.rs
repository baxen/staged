//! AI tool discovery and execution.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::prompt::{build_prompt_with_strategy, FileAnalysisInput, LARGE_FILE_THRESHOLD};
use super::types::ChangesetAnalysis;
use crate::git::{self, DiffSpec, FileContent};

/// Supported AI CLI tools.
#[derive(Debug, Clone)]
pub enum AiTool {
    Goose(PathBuf),
    Claude(PathBuf),
}

impl AiTool {
    pub fn name(&self) -> &'static str {
        match self {
            AiTool::Goose(_) => "goose",
            AiTool::Claude(_) => "claude",
        }
    }
}

/// Find an available AI CLI tool.
pub fn find_ai_tool() -> Option<AiTool> {
    if let Some(path) = find_in_path("goose") {
        return Some(AiTool::Goose(path));
    }
    if let Some(path) = find_in_path("claude") {
        return Some(AiTool::Claude(path));
    }
    None
}

fn find_in_path(cmd: &str) -> Option<PathBuf> {
    let output = Command::new("which").arg(cmd).output().ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    None
}

/// Check if output contains a context window error.
///
/// We need to be careful here - the AI's response might legitimately mention
/// "context window" when analyzing code that deals with context windows.
/// So we look for specific error phrases, not just any mention.
fn detect_context_error(output: &str, tool: &AiTool) -> Option<String> {
    let output_lower = output.to_lowercase();

    // These patterns should be specific error messages, not general phrases
    // that might appear in code or analysis
    let error_patterns: &[&str] = match tool {
        AiTool::Goose(_) => &[
            "context limit reached",
            "context length exceeded",
            "maximum context length exceeded",
            "prompt is too long",
            "input too long",
        ],
        AiTool::Claude(_) => &[
            "context length exceeded",
            "prompt is too long",
            "input too long",
            "exceeds the maximum number of tokens",
            "maximum context length",
        ],
    };

    for pattern in error_patterns {
        if output_lower.contains(pattern) {
            return Some(
                "Changeset too large for AI analysis. \
                 Try analyzing fewer files or a smaller diff range."
                    .to_string(),
            );
        }
    }
    None
}

fn run_tool(tool: &AiTool, prompt: &str) -> Result<String, String> {
    let output = match tool {
        AiTool::Goose(path) => Command::new(path)
            .args(["run", "-t", prompt])
            .output()
            .map_err(|e| format!("Failed to run goose: {}", e))?,

        AiTool::Claude(path) => Command::new(path)
            .args(["--dangerously-skip-permissions", "-p", prompt])
            .output()
            .map_err(|e| format!("Failed to run claude: {}", e))?,
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Check for context window errors in both stdout and stderr
    if let Some(error_msg) = detect_context_error(&stdout, tool) {
        return Err(error_msg);
    }
    if let Some(error_msg) = detect_context_error(&stderr, tool) {
        return Err(error_msg);
    }

    if !output.status.success() {
        return Err(format!(
            "{} failed (exit {}): {}",
            tool.name(),
            output.status.code().unwrap_or(-1),
            stderr
        ));
    }

    Ok(stdout)
}

/// Parse AI response into ChangesetAnalysis
fn parse_response(response: &str) -> Result<ChangesetAnalysis, String> {
    let response = response.trim();
    let json_str = extract_json(response);

    serde_json::from_str(json_str).map_err(|e| {
        log::error!("Failed to parse response as JSON: {}", e);
        log::error!("Response was:\n{}", response);
        format!("Failed to parse AI response: {}", e)
    })
}

/// Load after content for a file if it's small enough.
/// Returns None for deleted files, binary files, or files exceeding the threshold.
fn load_after_content_if_small(
    repo_path: &Path,
    spec: &DiffSpec,
    file_path: &Path,
) -> Result<(Option<String>, usize), String> {
    let diff = git::get_file_diff(repo_path, spec, file_path)
        .map_err(|e| format!("Failed to get file diff: {}", e))?;

    let (content, line_count) = match &diff.after {
        Some(f) => match &f.content {
            FileContent::Text { lines } => {
                let count = lines.len();
                if count <= LARGE_FILE_THRESHOLD {
                    (Some(lines.join("\n")), count)
                } else {
                    // File too large, skip content but report line count
                    (None, count)
                }
            }
            FileContent::Binary => (None, 0),
        },
        None => (None, 0), // Deleted file
    };

    Ok((content, line_count))
}

/// Analyze a diff using AI.
///
/// This is the main entry point - it handles:
/// 1. Listing files in the diff
/// 2. Loading unified diffs and after content for each file
/// 3. Building an appropriately-sized prompt (with automatic tier selection)
/// 4. Running AI analysis
/// 5. Returning the complete result
///
/// The frontend just needs to provide the diff spec.
pub fn analyze_diff(repo_path: &Path, spec: &DiffSpec) -> Result<ChangesetAnalysis, String> {
    // Find AI tool first (fail fast)
    let tool = find_ai_tool().ok_or_else(|| {
        "No AI CLI found. Install one of:\n\
         - goose: https://github.com/block/goose\n\
         - claude: npm install -g @anthropic-ai/claude-code"
            .to_string()
    })?;

    // List files in the diff
    let files = git::list_diff_files(repo_path, spec)
        .map_err(|e| format!("Failed to list diff files: {}", e))?;

    if files.is_empty() {
        return Err("No files in diff to analyze".to_string());
    }

    // Build inputs for each file
    let mut inputs: Vec<FileAnalysisInput> = Vec::new();

    for file_summary in &files {
        let file_path = file_summary.path();
        let path_str = file_path.to_string_lossy().to_string();

        // Get unified diff
        let diff = git::get_unified_diff(repo_path, spec, file_path)
            .map_err(|e| format!("Failed to get diff for {}: {}", path_str, e))?;

        // Load after content if small enough
        let (after_content, after_line_count) =
            load_after_content_if_small(repo_path, spec, file_path)?;

        // Determine file status
        let is_new_file = file_summary.is_added();
        let is_deleted = file_summary.is_deleted();

        // Skip binary files (no diff and no content)
        if diff.is_empty() && after_content.is_none() && !is_new_file && !is_deleted {
            // Check if it's actually binary by looking at the file
            let file_diff = git::get_file_diff(repo_path, spec, file_path).ok();
            let is_binary = file_diff.is_some_and(|d| {
                matches!(
                    d.after.as_ref().map(|f| &f.content),
                    Some(FileContent::Binary)
                ) || matches!(
                    d.before.as_ref().map(|f| &f.content),
                    Some(FileContent::Binary)
                )
            });
            if is_binary {
                continue;
            }
        }

        inputs.push(FileAnalysisInput {
            path: path_str,
            diff,
            after_content,
            is_new_file,
            is_deleted,
            after_line_count,
        });
    }

    if inputs.is_empty() {
        return Err("No text files to analyze (all binary?)".to_string());
    }

    // Build prompt with automatic tier selection
    let (prompt, strategy) = build_prompt_with_strategy(&inputs);

    log::info!("=== DIFF ANALYSIS ===");
    log::info!("Files: {}", inputs.len());
    log::info!("Strategy: {:?}", strategy);
    log::info!("Using: {}", tool.name());
    log::debug!("Prompt:\n{}", prompt);

    let response = run_tool(&tool, &prompt)?;

    log::debug!("Raw response:\n{}", response);

    parse_response(&response)
}

fn extract_json(response: &str) -> &str {
    // Check for ```json ... ``` pattern
    if let Some(start) = response.find("```json") {
        let after_fence = &response[start + 7..];
        if let Some(end) = after_fence.find("```") {
            return after_fence[..end].trim();
        }
    }

    // Check for ``` ... ``` pattern (no language)
    if let Some(start) = response.find("```") {
        let after_fence = &response[start + 3..];
        if let Some(end) = after_fence.find("```") {
            return after_fence[..end].trim();
        }
    }

    // Try to find JSON object directly
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            return &response[start..=end];
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_plain() {
        let input =
            r#"{"summary": "test", "key_changes": [], "concerns": [], "file_annotations": {}}"#;
        assert_eq!(extract_json(input), input);
    }

    #[test]
    fn test_extract_json_with_fence() {
        let input = r#"Here's the analysis:
```json
{"summary": "test", "key_changes": [], "concerns": [], "file_annotations": {}}
```"#;
        assert_eq!(
            extract_json(input),
            r#"{"summary": "test", "key_changes": [], "concerns": [], "file_annotations": {}}"#
        );
    }

    #[test]
    fn test_find_in_path() {
        assert!(find_in_path("ls").is_some());
        assert!(find_in_path("nonexistent_command_xyz").is_none());
    }

    #[test]
    fn test_detect_context_error_goose() {
        let tool = AiTool::Goose(PathBuf::from("/usr/bin/goose"));

        assert!(detect_context_error("Error: context limit reached", &tool).is_some());
        assert!(detect_context_error("Error: prompt is too long", &tool).is_some());
        assert!(detect_context_error("Normal output here", &tool).is_none());
        // Should NOT match general mentions of "context window" in analysis
        assert!(detect_context_error("This code handles context window errors", &tool).is_none());
    }

    #[test]
    fn test_detect_context_error_claude() {
        let tool = AiTool::Claude(PathBuf::from("/usr/bin/claude"));

        assert!(detect_context_error("Error: context length exceeded", &tool).is_some());
        assert!(detect_context_error("Error: input too long", &tool).is_some());
        assert!(detect_context_error("Normal output here", &tool).is_none());
        // Should NOT match general mentions in analysis
        assert!(detect_context_error("The code checks for token limits", &tool).is_none());
    }
}
