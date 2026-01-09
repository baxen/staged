//! AI-powered hunk description using goose.

use std::process::Command;

/// Describes a code change using goose AI.
///
/// Takes the before/after content of a hunk and the file path,
/// calls `goose run` with a prompt to describe the change.
pub fn describe_hunk(
    file_path: &str,
    before_lines: &[String],
    after_lines: &[String],
) -> Result<String, String> {
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
        r#"Can you describe the change here:

File: {}

Before:
```
{}
```

After:
```
{}
```

Give a brief, clear description of what changed and why it might have been done. Be concise (1-3 sentences)."#,
        file_path, before_content, after_content
    );

    log::info!("=== GOOSE DESCRIBE HUNK ===");
    log::info!("File: {}", file_path);
    log::info!("Before ({} lines):\n{}", before_lines.len(), before_content);
    log::info!("After ({} lines):\n{}", after_lines.len(), after_content);
    log::info!("Prompt:\n{}", prompt);

    let output = Command::new("goose")
        .args(["run", "-t", &prompt])
        .output()
        .map_err(|e| format!("Failed to run goose: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    log::info!("=== GOOSE RESPONSE ===");
    log::info!("Exit code: {:?}", output.status.code());
    log::info!("Stdout:\n{}", stdout);
    if !stderr.is_empty() {
        log::info!("Stderr:\n{}", stderr);
    }

    if !output.status.success() {
        return Err(format!(
            "goose exited with code {:?}: {}",
            output.status.code(),
            stderr
        ));
    }

    Ok(stdout.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires goose to be installed
    fn test_describe_hunk() {
        let before = vec!["fn old() {}".to_string()];
        let after = vec!["fn new_name() {}".to_string()];

        let result = describe_hunk("test.rs", &before, &after);
        println!("Result: {:?}", result);
        assert!(result.is_ok());
    }
}
