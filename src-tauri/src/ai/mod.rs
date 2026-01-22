//! AI-powered diff analysis.
//!
//! Shells out to AI CLI tools (goose or claude) to generate contextual
//! annotations for code changes.

mod prompt;
mod runner;
mod types;

pub use runner::{analyze_diff, find_ai_tool, AiTool};
pub use types::{ChangesetAnalysis, ChangesetSummary, SmartDiffAnnotation, SmartDiffResult};
