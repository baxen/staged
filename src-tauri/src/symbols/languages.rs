//! Language detection and Tree-sitter grammar loading.

use std::path::Path;
use tree_sitter::Language;

/// Supported languages for symbol parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedLanguage {
    TypeScript,
    Tsx,
    JavaScript,
    Jsx,
    Rust,
    Python,
    Go,
    Java,
    C,
    Cpp,
    CSharp,
    Ruby,
    Json,
    Php,
    Bash,
    Html,
    Css,
}

impl SupportedLanguage {
    /// Detect language from file path extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?;
        Self::from_extension(ext)
    }

    /// Detect language from file extension string.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "ts" | "mts" | "cts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "js" | "mjs" | "cjs" => Some(Self::JavaScript),
            "jsx" => Some(Self::Jsx),
            "rs" => Some(Self::Rust),
            "py" | "pyi" | "pyw" => Some(Self::Python),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "c" | "h" => Some(Self::C),
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" | "c++" | "h++" => Some(Self::Cpp),
            "cs" => Some(Self::CSharp),
            "rb" | "rake" | "gemspec" => Some(Self::Ruby),
            "json" | "jsonc" => Some(Self::Json),
            "php" | "phtml" | "php3" | "php4" | "php5" | "phps" => Some(Self::Php),
            "sh" | "bash" | "zsh" | "fish" => Some(Self::Bash),
            "html" | "htm" | "xhtml" => Some(Self::Html),
            "css" | "scss" | "sass" | "less" => Some(Self::Css),
            _ => None,
        }
    }

    /// Get the Tree-sitter language for this language type.
    pub fn tree_sitter_language(&self) -> Language {
        match self {
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::JavaScript | Self::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Self::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Self::Ruby => tree_sitter_ruby::LANGUAGE.into(),
            Self::Json => tree_sitter_json::LANGUAGE.into(),
            Self::Php => tree_sitter_php::LANGUAGE_PHP.into(),
            Self::Bash => tree_sitter_bash::LANGUAGE.into(),
            Self::Html => tree_sitter_html::LANGUAGE.into(),
            Self::Css => tree_sitter_css::LANGUAGE.into(),
        }
    }

    /// Get definition node types for this language.
    /// These are AST node types that can define symbols.
    pub fn definition_node_types(&self) -> &'static [&'static str] {
        match self {
            Self::TypeScript | Self::Tsx => &[
                "function_declaration",
                "method_definition",
                "class_declaration",
                "interface_declaration",
                "type_alias_declaration",
                "enum_declaration",
                "variable_declarator",
                "lexical_declaration",
                "export_statement",
                "import_statement",
                "arrow_function",
            ],
            Self::JavaScript | Self::Jsx => &[
                "function_declaration",
                "method_definition",
                "class_declaration",
                "variable_declarator",
                "lexical_declaration",
                "export_statement",
                "import_statement",
                "arrow_function",
            ],
            Self::Rust => &[
                "function_item",
                "struct_item",
                "enum_item",
                "type_item",
                "trait_item",
                "impl_item",
                "mod_item",
                "const_item",
                "static_item",
                "macro_definition",
                "use_declaration",
            ],
            Self::Python => &[
                "function_definition",
                "class_definition",
                "assignment",
                "import_statement",
                "import_from_statement",
            ],
            Self::Go => &[
                "function_declaration",
                "method_declaration",
                "type_declaration",
                "const_declaration",
                "var_declaration",
                "import_declaration",
            ],
            Self::Java => &[
                "method_declaration",
                "class_declaration",
                "interface_declaration",
                "enum_declaration",
                "field_declaration",
                "import_declaration",
            ],
            Self::C | Self::Cpp => &[
                "function_definition",
                "declaration",
                "struct_specifier",
                "enum_specifier",
                "type_definition",
            ],
            Self::CSharp => &[
                "method_declaration",
                "class_declaration",
                "interface_declaration",
                "struct_declaration",
                "enum_declaration",
                "field_declaration",
                "property_declaration",
            ],
            Self::Ruby => &[
                "method",
                "singleton_method",
                "class",
                "module",
                "assignment",
            ],
            Self::Json => &[], // JSON doesn't have definitions
            Self::Php => &[
                "function_definition",
                "method_declaration",
                "class_declaration",
                "interface_declaration",
                "trait_declaration",
                "const_declaration",
                "property_declaration",
            ],
            Self::Bash => &[
                "function_definition",
                "variable_assignment",
            ],
            Self::Html => &[], // HTML doesn't have traditional definitions
            Self::Css => &[
                "rule_set",
                "keyframes_statement",
            ],
        }
    }

    /// Get identifier node types for this language.
    /// These nodes contain symbol names that can be clicked.
    pub fn identifier_node_types(&self) -> &'static [&'static str] {
        match self {
            Self::TypeScript | Self::Tsx | Self::JavaScript | Self::Jsx => &[
                "identifier",
                "property_identifier",
                "type_identifier",
                "shorthand_property_identifier",
            ],
            Self::Rust => &[
                "identifier",
                "type_identifier",
                "field_identifier",
                "scoped_identifier",
            ],
            Self::Python => &["identifier"],
            Self::Go => &["identifier", "type_identifier", "field_identifier"],
            Self::Java => &["identifier", "type_identifier"],
            Self::C | Self::Cpp => &["identifier", "type_identifier", "field_identifier"],
            Self::CSharp => &["identifier"],
            Self::Ruby => &["identifier", "constant"],
            Self::Json => &["string"], // Keys in JSON
            Self::Php => &["name", "variable_name"],
            Self::Bash => &["variable_name", "word"],
            Self::Html => &["tag_name", "attribute_name"],
            Self::Css => &["class_name", "id_name", "property_name"],
        }
    }

    /// Get the name of the language for display purposes.
    pub fn name(&self) -> &'static str {
        match self {
            Self::TypeScript => "TypeScript",
            Self::Tsx => "TSX",
            Self::JavaScript => "JavaScript",
            Self::Jsx => "JSX",
            Self::Rust => "Rust",
            Self::Python => "Python",
            Self::Go => "Go",
            Self::Java => "Java",
            Self::C => "C",
            Self::Cpp => "C++",
            Self::CSharp => "C#",
            Self::Ruby => "Ruby",
            Self::Json => "JSON",
            Self::Php => "PHP",
            Self::Bash => "Bash",
            Self::Html => "HTML",
            Self::Css => "CSS",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_language_from_extension() {
        assert_eq!(
            SupportedLanguage::from_extension("ts"),
            Some(SupportedLanguage::TypeScript)
        );
        assert_eq!(
            SupportedLanguage::from_extension("tsx"),
            Some(SupportedLanguage::Tsx)
        );
        assert_eq!(
            SupportedLanguage::from_extension("js"),
            Some(SupportedLanguage::JavaScript)
        );
        assert_eq!(
            SupportedLanguage::from_extension("rs"),
            Some(SupportedLanguage::Rust)
        );
        assert_eq!(
            SupportedLanguage::from_extension("py"),
            Some(SupportedLanguage::Python)
        );
        assert_eq!(
            SupportedLanguage::from_extension("go"),
            Some(SupportedLanguage::Go)
        );
        assert_eq!(SupportedLanguage::from_extension("unknown"), None);
    }

    #[test]
    fn test_language_from_path() {
        assert_eq!(
            SupportedLanguage::from_path(&PathBuf::from("src/main.ts")),
            Some(SupportedLanguage::TypeScript)
        );
        assert_eq!(
            SupportedLanguage::from_path(&PathBuf::from("lib/utils.py")),
            Some(SupportedLanguage::Python)
        );
        assert_eq!(
            SupportedLanguage::from_path(&PathBuf::from("README.md")),
            None
        );
    }
}
