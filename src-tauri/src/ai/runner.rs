//! AI tool discovery and execution.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::prompt::build_unified_changeset_prompt;
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

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{} failed (exit {}): {}",
            tool.name(),
            output.status.code().unwrap_or(-1),
            stderr
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Analyze an entire changeset using AI.
fn parse_response(response: &str) -> Result<ChangesetAnalysis, String> {
    let response = response.trim();
    let json_str = extract_json(response);

    serde_json::from_str(json_str).map_err(|e| {
        log::error!("Failed to parse response as JSON: {}", e);
        log::error!("Response was:\n{}", response);
        format!("Failed to parse AI response: {}", e)
    })
}

/// Analyze a diff using AI.
///
/// This is the main entry point - it handles:
/// 1. Listing files in the diff
/// 2. Loading before/after content for each file
/// 3. Running AI analysis
/// 4. Returning the complete result
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

    // Load content for each file
    let mut file_contents: Vec<(String, String, String)> = Vec::new();

    for file_summary in &files {
        let file_path = file_summary.path();
        let path_str = file_path.to_string_lossy().to_string();

        let diff = git::get_file_diff(repo_path, spec, file_path)
            .map_err(|e| format!("Failed to get diff for {}: {}", path_str, e))?;

        // Extract text content
        let before_content = match &diff.before {
            Some(f) => match &f.content {
                FileContent::Text { lines } => lines.join("\n"),
                FileContent::Binary => String::new(),
            },
            None => String::new(),
        };

        let after_content = match &diff.after {
            Some(f) => match &f.content {
                FileContent::Text { lines } => lines.join("\n"),
                FileContent::Binary => String::new(),
            },
            None => String::new(),
        };

        // Skip binary files (both exist but no text content)
        if before_content.is_empty()
            && after_content.is_empty()
            && (diff.before.is_some() || diff.after.is_some())
        {
            continue;
        }

        file_contents.push((path_str, before_content, after_content));
    }

    if file_contents.is_empty() {
        return Err("No text files to analyze (all binary?)".to_string());
    }

    // Build prompt and run AI
    let file_refs: Vec<(&str, &str, &str)> = file_contents
        .iter()
        .map(|(p, b, a)| (p.as_str(), b.as_str(), a.as_str()))
        .collect();

    let prompt = build_unified_changeset_prompt(&file_refs);

    log::info!("=== DIFF ANALYSIS ===");
    log::info!("Files: {}", file_refs.len());
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
}
