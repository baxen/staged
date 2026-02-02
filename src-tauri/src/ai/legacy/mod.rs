//! Legacy diff analysis module.
//!
//! Contains the original AI-powered diff analysis implementation.
//! This is being kept for reference while we consolidate to a single
//! ACP integration point.

mod prompt;
mod runner;
mod types;

pub use prompt::{build_prompt_with_strategy, FileAnalysisInput, PromptStrategy};
pub use runner::analyze_diff;
pub use types::{
    AnnotationCategory, ChangesetAnalysis, ChangesetSummary, SmartDiffAnnotation, SmartDiffResult,
};
