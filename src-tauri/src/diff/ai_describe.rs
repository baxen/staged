//! AI-powered hunk description using goose or claude.

use std::path::PathBuf;
use std::process::Command;

/// Result of describing a hunk - before and after description in natural language
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HunkDescription {
    pub before: String,
    pub after: String,
}

/// Common paths where CLI tools might be installed, needed when running app in packaged form (vs justfile)
const CLI_SEARCH_PATHS: &[&str] = &[
    "/usr/local/bin",
    "/opt/homebrew/bin",
    "/home/linuxbrew/.linuxbrew/bin",
    "/usr/bin",
];

#[derive(Debug)]
enum AiTool {
    Goose(PathBuf),
    Claude(PathBuf),
}

/// Find the `goose` CLI executable.
/// Checks PATH first, then falls back to common installation locations.
fn find_goose_command() -> Option<PathBuf> {
    // First, check if `goose` is directly available (e.g., already in PATH)
    if let Ok(output) = Command::new("goose").arg("--version").output() {
        if output.status.success() {
            return Some(PathBuf::from("goose"));
        }
    }

    // Check common installation paths
    for dir in CLI_SEARCH_PATHS {
        let path = PathBuf::from(dir).join("goose");
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Find the `claude` CLI executable.
/// Checks PATH first, then falls back to common installation locations.
fn find_claude_command() -> Option<PathBuf> {
    // First, check if `claude` is directly available (e.g., already in PATH)
    if let Ok(output) = Command::new("claude").arg("--version").output() {
        if output.status.success() {
            return Some(PathBuf::from("claude"));
        }
    }

    // Check common installation paths
    for dir in CLI_SEARCH_PATHS {
        let path = PathBuf::from(dir).join("claude");
        if path.exists() {
            return Some(path);
        }
    }

    None
}

fn find_ai_tool() -> Option<AiTool> {
    if let Some(path) = find_goose_command() {
        return Some(AiTool::Goose(path));
    }
    if let Some(path) = find_claude_command() {
        return Some(AiTool::Claude(path));
    }
    None
}

fn run_ai_tool(tool: &AiTool, prompt: &str) -> Result<String, String> {
    let output = match tool {
        AiTool::Goose(path) => {
            log::info!("Using goose at: {:?}", path);
            Command::new(path)
                .args(["run", "-t", prompt])
                .output()
                .map_err(|e| format!("Failed to run goose: {}", e))?
        }
        AiTool::Claude(path) => {
            log::info!("Using claude at: {:?}", path);
            Command::new(path)
                .args(["--dangerously-skip-permissions", "-p", prompt])
                .output()
                .map_err(|e| format!("Failed to run claude: {}", e))?
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    log::info!("=== AI RESPONSE ===");
    log::info!("Exit code: {:?}", output.status.code());
    log::info!("Stdout:\n{}", stdout);
    if !stderr.is_empty() {
        log::info!("Stderr:\n{}", stderr);
    }

    if !output.status.success() {
        let tool_name = match tool {
            AiTool::Goose(_) => "goose",
            AiTool::Claude(_) => "claude",
        };
        return Err(format!(
            "{} exited with code {:?}: {}",
            tool_name,
            output.status.code(),
            stderr
        ));
    }

    Ok(stdout)
}

/// Describes a code change using goose AI (or claude as fallback).
///
pub fn describe_hunk(
    file_path: &str,
    before_lines: &[String],
    after_lines: &[String],
) -> Result<HunkDescription, String> {
    let tool = find_ai_tool().ok_or_else(|| {
        "No AI CLI found. Install one of:\n           - goose: brew install goose or see https://github.com/block/goose\n           - claude: npm install -g @anthropic-ai/claude-code"
            .to_string()
    })?;

    let before_content = if before_lines.is_empty() {
        "(empty - new content)".to_string()
    } else {
        before_lines.join("\n")
    };

    let after_content = if after_lines.is_empty() {
        "(empty - deleted content)".to_string()
    } else {
        after_lines.join("\n")
    };

    let prompt = format!(
        r#"Describe this code change concisely. Output EXACTLY in this format with no other text:

BEFORE: <one line describing what the old code did>
AFTER: <one line describing what the new code does>

File: {}

Old code:
```
{}
```

New code:
```
{}
```"#,
        file_path, before_content, after_content
    );

    log::info!("=== AI DESCRIBE HUNK ===");
    log::info!("File: {}", file_path);
    log::info!("Prompt:\n{}", prompt);

    let response = run_ai_tool(&tool, &prompt)?;

    // Parse the response - look for BEFORE: and AFTER: lines
    let response = response.trim();
    let before_desc = extract_field(response, "BEFORE:")
        .unwrap_or_else(|| "Could not parse before description".to_string());
    let after_desc = extract_field(response, "AFTER:")
        .unwrap_or_else(|| "Could not parse after description".to_string());

    Ok(HunkDescription {
        before: before_desc,
        after: after_desc,
    })
}

fn extract_field(response: &str, field: &str) -> Option<String> {
    for line in response.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix(field) {
            return Some(value.trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_field() {
        let response = "BEFORE: old behavior\nAFTER: new behavior";
        assert_eq!(
            extract_field(response, "BEFORE:"),
            Some("old behavior".to_string())
        );
        assert_eq!(
            extract_field(response, "AFTER:"),
            Some("new behavior".to_string())
        );
    }

    #[test]
    #[ignore]
    fn test_describe_hunk() {
        let before = vec!["fn old() {}".to_string()];
        let after = vec!["fn new_name() {}".to_string()];

        let result = describe_hunk("test.rs", &before, &after);
        println!("Result: {:?}", result);
        assert!(result.is_ok());
    }
}
