//! Symbol parsing and definition finding.
//!
//! Uses Tree-sitter to parse source code and find symbol definitions.
//! Supports multiple languages (TypeScript, JavaScript, Rust, Python, Go, etc.)

mod definition;
mod languages;
mod parser;

pub use definition::{find_definition, DefinitionResult};
pub use parser::{get_symbol_at_position, SymbolInfo, SymbolKind};

use languages::SupportedLanguage;
use std::path::Path;

/// Check if a file extension is supported for symbol navigation.
pub fn is_supported_extension(path: &Path) -> bool {
    SupportedLanguage::from_path(path).is_some()
}
