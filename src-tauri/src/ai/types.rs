//! Types for smart diff annotations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A span of lines (0-indexed, exclusive end).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineSpan {
    pub start: usize,
    pub end: usize,
}

/// Category of annotation to help with display styling.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationCategory {
    /// Explains what the code does
    #[default]
    Explanation,
    /// Highlights a potential issue
    Warning,
    /// Suggests an improvement
    Suggestion,
    /// Provides background context
    Context,
}

/// A single annotation on a diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartDiffAnnotation {
    /// Unique identifier for this annotation
    pub id: String,

    /// Description of the old state (for before_span annotations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_description: Option<String>,

    /// File path this annotation belongs to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,

    /// Span in the 'before' content (None if only applies to 'after')
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_span: Option<LineSpan>,

    /// Span in the 'after' content (None if only applies to 'before')
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_span: Option<LineSpan>,

    /// The AI commentary
    pub content: String,

    /// Category for styling
    #[serde(default)]
    pub category: AnnotationCategory,
}

/// Per-file analysis result (used for storage and display).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartDiffResult {
    /// Summary (unused in unified model, kept for storage compatibility)
    pub overview: String,

    /// Span-based annotations for this file
    pub annotations: Vec<SmartDiffAnnotation>,
}

impl SmartDiffResult {
    /// Create an empty result.
    pub fn empty() -> Self {
        Self {
            overview: String::new(),
            annotations: Vec::new(),
        }
    }
}

/// Summary portion of changeset analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangesetSummary {
    /// High-level summary of what this changeset accomplishes
    pub summary: String,
    /// Key changes organized by theme/area
    pub key_changes: Vec<String>,
    /// Potential concerns or things to watch out for
    pub concerns: Vec<String>,
}

/// Complete analysis of an entire changeset.
/// The AI sees all files together and produces both summary and annotations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangesetAnalysis {
    /// High-level summary of what this changeset accomplishes
    pub summary: String,
    /// Key changes organized by theme/area
    pub key_changes: Vec<String>,
    /// Potential concerns or things to watch out for
    pub concerns: Vec<String>,
    /// Annotations keyed by file path
    pub file_annotations: HashMap<String, Vec<SmartDiffAnnotation>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_changeset_analysis() {
        let json = r#"{
            "summary": "Added new feature",
            "key_changes": ["New API endpoint", "Updated tests"],
            "concerns": [],
            "file_annotations": {
                "src/api.rs": [
                    {
                        "id": "1",
                        "file_path": "src/api.rs",
                        "after_span": {"start": 5, "end": 15},
                        "content": "New endpoint handler",
                        "category": "explanation"
                    }
                ]
            }
        }"#;

        let result: ChangesetAnalysis = serde_json::from_str(json).unwrap();
        assert_eq!(result.summary, "Added new feature");
        assert_eq!(result.key_changes.len(), 2);
        assert!(result.file_annotations.contains_key("src/api.rs"));
    }
}
