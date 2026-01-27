//! Prompt template for AI diff analysis.

/// Format content with line numbers for the AI to reference.
fn format_with_line_numbers(content: &str) -> String {
    content
        .lines()
        .enumerate()
        .map(|(i, line)| format!("{:4} | {}", i, line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Build the prompt for analyzing an entire changeset with full file contents.
pub fn build_unified_changeset_prompt(files: &[(&str, &str, &str)]) -> String {
    let mut file_sections = String::new();

    for (path, before, after) in files {
        file_sections.push_str(&format!("\n## File: {}\n\n", path));

        if before.is_empty() {
            file_sections.push_str("### BEFORE:\n(new file - no previous content)\n\n");
        } else {
            file_sections.push_str("### BEFORE:\n```\n");
            file_sections.push_str(&format_with_line_numbers(before));
            file_sections.push_str("\n```\n\n");
        }

        if after.is_empty() {
            file_sections.push_str("### AFTER:\n(deleted file - no new content)\n\n");
        } else {
            file_sections.push_str("### AFTER:\n```\n");
            file_sections.push_str(&format_with_line_numbers(after));
            file_sections.push_str("\n```\n\n");
        }
    }

    format!(
        r#"{SYSTEM_PROMPT}

# Changeset ({file_count} files)
{file_sections}

{OUTPUT_FORMAT}"#,
        SYSTEM_PROMPT = SYSTEM_PROMPT,
        file_count = files.len(),
        file_sections = file_sections,
        OUTPUT_FORMAT = OUTPUT_FORMAT,
    )
}

const SYSTEM_PROMPT: &str = r#"You are a code review assistant. Analyze the following changeset (all files together) and provide:

1. A high-level summary of what this changeset accomplishes
2. Key changes organized by theme (2-5 bullet points)
3. Any concerns worth noting (0-3 items, empty if none)
4. Annotations on specific code sections that deserve commentary

**Important guidelines**:
- You see the FULL context of all files - use this to understand cross-file relationships
- Annotations should tell the story of the change, not exhaustively document every line
- Focus on what matters: the "why", potential issues, non-obvious implications
- It's fine to have no annotations for trivial or self-explanatory files
- Reference line numbers from the numbered content shown (0-indexed)
"#;

const OUTPUT_FORMAT: &str = r#"## Output Format

Respond with ONLY valid JSON matching this structure (no markdown code fences, no other text):

{
  "summary": "2-3 sentence high-level summary of what this changeset accomplishes",
  "key_changes": [
    "First major change or theme",
    "Second major change or theme"
  ],
  "concerns": [
    "Any potential issue or area needing careful review"
  ],
  "file_annotations": {
    "path/to/file.rs": [
      {
        "id": "1",
        "file_path": "path/to/file.rs",
        "before_span": {"start": 8, "end": 15},
        "before_description": "Previously handled errors by panicking",
        "after_span": {"start": 10, "end": 20},
        "content": "Your commentary on this section",
        "category": "explanation"
      }
    ],
    "path/to/other.ts": []
  }
}

Rules:
- "summary": Brief overview suitable for a PR description
- "key_changes": 2-5 bullet points grouping related changes
- "concerns": 0-3 potential issues (empty array if none)
- "file_annotations": Object with file paths as keys, arrays of annotations as values
  - Include ALL files from the changeset as keys (use empty array [] if no annotations needed)
  - "id": Unique across ALL annotations (use "1", "2", "3", etc.)
  - "file_path": Must match the key exactly
  - "before_span": Line range in BEFORE content (0-indexed, exclusive end). Omit if only about new code.
  - "before_description": When before_span is provided, describe what the old code was doing (1 sentence). Required if before_span is set.
  - "after_span": Line range in AFTER content (0-indexed, exclusive end). Omit if only about deleted code.
  - "content": Your commentary (1-3 sentences)
  - "category": One of "explanation", "warning", "suggestion", "context""#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_unified_changeset_prompt() {
        let files = vec![
            ("src/main.rs", "fn old() {}", "fn new() {}"),
            ("src/lib.rs", "", "pub mod api;"),
        ];

        let prompt = build_unified_changeset_prompt(&files);

        assert!(prompt.contains("Changeset (2 files)"));
        assert!(prompt.contains("File: src/main.rs"));
        assert!(prompt.contains("File: src/lib.rs"));
        assert!(prompt.contains("file_annotations"));
    }
}
