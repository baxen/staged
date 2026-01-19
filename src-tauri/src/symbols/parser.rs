//! Tree-sitter parsing and symbol extraction.

use super::languages::SupportedLanguage;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tree_sitter::{Node, Parser, Tree};

/// The kind of symbol (function, class, variable, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Interface,
    Type,
    Enum,
    Variable,
    Constant,
    Module,
    Import,
    Property,
    Unknown,
}

/// Information about a symbol at a position.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolInfo {
    /// The symbol name
    pub name: String,
    /// The kind of symbol
    pub kind: SymbolKind,
    /// Line number (0-indexed)
    pub line: usize,
    /// Column number (0-indexed)
    pub column: usize,
    /// End column (0-indexed)
    pub end_column: usize,
    /// The full text of the node containing this symbol (for context)
    pub context: Option<String>,
    /// Language of the file
    pub language: String,
}

/// Parse source code and build a Tree-sitter tree.
pub fn parse_source(content: &str, language: SupportedLanguage) -> Option<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&language.tree_sitter_language())
        .ok()?;
    parser.parse(content, None)
}

/// Get the symbol at a specific position in the source code.
///
/// # Arguments
/// * `file_path` - Path to the file (used to detect language)
/// * `content` - The source code content
/// * `line` - Line number (0-indexed)
/// * `column` - Column number (0-indexed)
///
/// # Returns
/// `Some(SymbolInfo)` if a symbol was found at the position, `None` otherwise.
pub fn get_symbol_at_position(
    file_path: &Path,
    content: &str,
    line: usize,
    column: usize,
) -> Option<SymbolInfo> {
    let language = SupportedLanguage::from_path(file_path)?;
    let tree = parse_source(content, language)?;
    let root = tree.root_node();

    // Find the smallest node at the position
    let point = tree_sitter::Point::new(line, column);
    let node = find_smallest_node_at_point(root, point)?;

    // Reject if we're inside a string literal or comment
    if is_inside_string_or_comment(node) {
        return None;
    }

    // Check if this is an identifier node
    let identifier_types = language.identifier_node_types();
    if !identifier_types.contains(&node.kind()) {
        // Try to find an identifier child
        if let Some(id_node) = find_identifier_in_node(node, identifier_types) {
            return extract_symbol_info(id_node, content, language);
        }
        return None;
    }

    extract_symbol_info(node, content, language)
}

/// Check if a node is inside a string literal or comment.
fn is_inside_string_or_comment(node: Node) -> bool {
    let non_navigable_types = [
        // Strings
        "string",
        "string_literal",
        "string_fragment",
        "template_string",
        "template_literal",
        "raw_string_literal",
        "char_literal",
        "interpreted_string_literal",
        "rune_literal",
        // Comments
        "comment",
        "line_comment",
        "block_comment",
        "doc_comment",
    ];

    // Check the node itself
    if non_navigable_types.contains(&node.kind()) {
        return true;
    }

    // Check parent nodes
    let mut current = node.parent();
    while let Some(parent) = current {
        if non_navigable_types.contains(&parent.kind()) {
            return true;
        }
        current = parent.parent();
    }

    false
}

/// Find all symbols in a file.
pub fn find_all_symbols(file_path: &Path, content: &str) -> Vec<SymbolInfo> {
    let Some(language) = SupportedLanguage::from_path(file_path) else {
        return vec![];
    };
    let Some(tree) = parse_source(content, language) else {
        return vec![];
    };

    let mut symbols = Vec::new();
    let definition_types = language.definition_node_types();

    collect_definitions(tree.root_node(), content, language, definition_types, &mut symbols);
    symbols
}

/// Collect all definition nodes recursively.
fn collect_definitions(
    node: Node,
    content: &str,
    language: SupportedLanguage,
    definition_types: &[&str],
    symbols: &mut Vec<SymbolInfo>,
) {
    if definition_types.contains(&node.kind()) {
        if let Some(symbol) = extract_definition_symbol(node, content, language) {
            symbols.push(symbol);
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_definitions(child, content, language, definition_types, symbols);
    }
}

/// Extract symbol info from a definition node.
fn extract_definition_symbol(
    node: Node,
    content: &str,
    language: SupportedLanguage,
) -> Option<SymbolInfo> {
    let identifier_types = language.identifier_node_types();

    // Find the name identifier within this definition
    let name_node = find_name_in_definition(node, identifier_types, language)?;
    let name = name_node.utf8_text(content.as_bytes()).ok()?;

    let kind = determine_symbol_kind(node.kind(), language);

    Some(SymbolInfo {
        name: name.to_string(),
        kind,
        line: name_node.start_position().row,
        column: name_node.start_position().column,
        end_column: name_node.end_position().column,
        context: Some(get_context_line(content, name_node.start_position().row)),
        language: language.name().to_string(),
    })
}

/// Find the name identifier within a definition node.
fn find_name_in_definition<'a>(
    node: Node<'a>,
    identifier_types: &[&str],
    language: SupportedLanguage,
) -> Option<Node<'a>> {
    // Different strategies based on language and node type
    match language {
        SupportedLanguage::TypeScript
        | SupportedLanguage::Tsx
        | SupportedLanguage::JavaScript
        | SupportedLanguage::Jsx => find_js_definition_name(node, identifier_types),
        SupportedLanguage::Rust => find_rust_definition_name(node, identifier_types),
        SupportedLanguage::Python => find_python_definition_name(node, identifier_types),
        SupportedLanguage::Go => find_go_definition_name(node, identifier_types),
        _ => find_first_identifier(node, identifier_types),
    }
}

/// Find definition name for JS/TS.
fn find_js_definition_name<'a>(node: Node<'a>, identifier_types: &[&str]) -> Option<Node<'a>> {
    let kind = node.kind();

    // For variable declarations, look for the name in the declarator
    if kind == "lexical_declaration" || kind == "variable_declaration" {
        if let Some(declarator) = node.child_by_field_name("declarator") {
            return declarator.child_by_field_name("name");
        }
        // Try first child if no declarator field
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(name) = child.child_by_field_name("name") {
                    return Some(name);
                }
            }
        }
    }

    // For function/class declarations, use the name field
    if let Some(name) = node.child_by_field_name("name") {
        return Some(name);
    }

    // For export statements, look inside
    if kind == "export_statement" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(name) = find_js_definition_name(child, identifier_types) {
                return Some(name);
            }
        }
    }

    find_first_identifier(node, identifier_types)
}

/// Find definition name for Rust.
fn find_rust_definition_name<'a>(node: Node<'a>, identifier_types: &[&str]) -> Option<Node<'a>> {
    // Rust uses "name" field for most definitions
    if let Some(name) = node.child_by_field_name("name") {
        return Some(name);
    }

    // For use declarations, find the identifier
    if node.kind() == "use_declaration" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(id) = find_rust_use_name(child, identifier_types) {
                return Some(id);
            }
        }
    }

    find_first_identifier(node, identifier_types)
}

/// Find the name in a Rust use declaration.
fn find_rust_use_name<'a>(node: Node<'a>, identifier_types: &[&str]) -> Option<Node<'a>> {
    if identifier_types.contains(&node.kind()) {
        return Some(node);
    }

    // Handle scoped identifiers - get the last component
    if node.kind() == "scoped_identifier" || node.kind() == "use_wildcard" {
        if let Some(name) = node.child_by_field_name("name") {
            return Some(name);
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_rust_use_name(child, identifier_types) {
            return Some(found);
        }
    }
    None
}

/// Find definition name for Python.
fn find_python_definition_name<'a>(node: Node<'a>, identifier_types: &[&str]) -> Option<Node<'a>> {
    // Python uses "name" field for function/class definitions
    if let Some(name) = node.child_by_field_name("name") {
        return Some(name);
    }

    // For assignments, get the left side
    if node.kind() == "assignment" {
        if let Some(left) = node.child_by_field_name("left") {
            if identifier_types.contains(&left.kind()) {
                return Some(left);
            }
        }
    }

    find_first_identifier(node, identifier_types)
}

/// Find definition name for Go.
fn find_go_definition_name<'a>(node: Node<'a>, identifier_types: &[&str]) -> Option<Node<'a>> {
    // Go uses "name" field for function declarations
    if let Some(name) = node.child_by_field_name("name") {
        return Some(name);
    }

    // For type declarations, look inside
    if node.kind() == "type_declaration" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_spec" {
                if let Some(name) = child.child_by_field_name("name") {
                    return Some(name);
                }
            }
        }
    }

    find_first_identifier(node, identifier_types)
}

/// Find the first identifier node in a subtree.
fn find_first_identifier<'a>(node: Node<'a>, identifier_types: &[&str]) -> Option<Node<'a>> {
    if identifier_types.contains(&node.kind()) {
        return Some(node);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_first_identifier(child, identifier_types) {
            return Some(found);
        }
    }
    None
}

/// Find the smallest node containing a point.
fn find_smallest_node_at_point(root: Node, point: tree_sitter::Point) -> Option<Node> {
    let mut cursor = root.walk();
    let mut smallest: Option<Node> = None;

    loop {
        let node = cursor.node();

        // Check if point is within this node
        if node.start_position() <= point && point < node.end_position() {
            smallest = Some(node);

            // Try to go deeper
            if cursor.goto_first_child() {
                continue;
            }
        }

        // Try next sibling
        if cursor.goto_next_sibling() {
            continue;
        }

        // Go up and try next sibling
        loop {
            if !cursor.goto_parent() {
                return smallest;
            }
            if cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// Find an identifier node within a node or its children.
fn find_identifier_in_node<'a>(node: Node<'a>, identifier_types: &[&str]) -> Option<Node<'a>> {
    if identifier_types.contains(&node.kind()) {
        return Some(node);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_identifier_in_node(child, identifier_types) {
            return Some(found);
        }
    }
    None
}

/// Extract symbol info from an identifier node.
fn extract_symbol_info(node: Node, content: &str, language: SupportedLanguage) -> Option<SymbolInfo> {
    let name = node.utf8_text(content.as_bytes()).ok()?;

    // Filter out string literals and invalid identifiers
    if !is_valid_identifier(name) {
        return None;
    }

    // Try to determine the kind from the parent context
    let kind = if let Some(parent) = node.parent() {
        determine_symbol_kind(parent.kind(), language)
    } else {
        SymbolKind::Unknown
    };

    Some(SymbolInfo {
        name: name.to_string(),
        kind,
        line: node.start_position().row,
        column: node.start_position().column,
        end_column: node.end_position().column,
        context: Some(get_context_line(content, node.start_position().row)),
        language: language.name().to_string(),
    })
}

/// Check if a name looks like a valid identifier (not a string literal or keyword).
fn is_valid_identifier(name: &str) -> bool {
    // Reject empty names
    if name.is_empty() {
        return false;
    }

    // Reject string literals (start/end with quotes)
    if name.starts_with('"')
        || name.starts_with('\'')
        || name.starts_with('`')
        || name.ends_with('"')
        || name.ends_with('\'')
        || name.ends_with('`')
    {
        return false;
    }

    // Reject names with spaces or invalid characters
    if name.contains(' ') || name.contains('\n') || name.contains('\t') {
        return false;
    }

    // Reject names that start with a digit
    if name.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return false;
    }

    // Reject common keywords that aren't navigable
    let keywords = [
        "if", "else", "for", "while", "do", "switch", "case", "break", "continue", "return",
        "try", "catch", "finally", "throw", "new", "delete", "typeof", "instanceof", "void",
        "this", "super", "null", "undefined", "true", "false", "in", "of", "with", "as",
        "async", "await", "yield", "let", "const", "var", "function", "class", "extends",
        "implements", "import", "export", "from", "default", "static", "public", "private",
        "protected", "readonly", "abstract", "interface", "type", "enum", "namespace", "module",
        // Rust keywords
        "fn", "pub", "mod", "use", "struct", "impl", "trait", "where", "mut", "ref", "self",
        "Self", "match", "loop", "move", "dyn", "unsafe", "extern", "crate",
        // Python keywords
        "def", "lambda", "pass", "raise", "global", "nonlocal", "assert", "del", "print",
        "exec", "eval", "and", "or", "not", "is", "None", "True", "False",
        // Go keywords
        "func", "package", "defer", "go", "select", "chan", "map", "range", "fallthrough",
        "goto",
    ];

    !keywords.contains(&name)
}

/// Determine the symbol kind from the AST node type.
fn determine_symbol_kind(node_type: &str, _language: SupportedLanguage) -> SymbolKind {
    match node_type {
        // Functions
        "function_declaration"
        | "function_definition"
        | "function_item"
        | "arrow_function" => SymbolKind::Function,

        // Methods
        "method_definition" | "method_declaration" | "method" | "singleton_method" => {
            SymbolKind::Method
        }

        // Classes
        "class_declaration" | "class_definition" | "class" | "struct_item" | "struct_specifier" => {
            SymbolKind::Class
        }

        // Interfaces
        "interface_declaration" | "trait_item" => SymbolKind::Interface,

        // Types
        "type_alias_declaration"
        | "type_item"
        | "type_declaration"
        | "type_definition"
        | "type_spec" => SymbolKind::Type,

        // Enums
        "enum_declaration" | "enum_item" | "enum_specifier" => SymbolKind::Enum,

        // Variables
        "variable_declarator"
        | "lexical_declaration"
        | "variable_declaration"
        | "assignment"
        | "var_declaration"
        | "declaration" => SymbolKind::Variable,

        // Constants
        "const_item" | "const_declaration" | "static_item" => SymbolKind::Constant,

        // Modules
        "mod_item" | "module" => SymbolKind::Module,

        // Imports
        "import_statement"
        | "import_declaration"
        | "import_from_statement"
        | "use_declaration" => SymbolKind::Import,

        // Properties
        "property_identifier"
        | "field_declaration"
        | "property_declaration"
        | "field_identifier" => SymbolKind::Property,

        // Default
        _ => SymbolKind::Unknown,
    }
}

/// Get a single line of context for display.
fn get_context_line(content: &str, line: usize) -> String {
    content
        .lines()
        .nth(line)
        .map(|l| l.trim().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_typescript() {
        let content = r#"
function greet(name: string): string {
    return `Hello, ${name}!`;
}

class Person {
    name: string;

    constructor(name: string) {
        this.name = name;
    }

    greet() {
        return greet(this.name);
    }
}
"#;
        let path = PathBuf::from("test.ts");
        let symbols = find_all_symbols(&path, content);

        assert!(!symbols.is_empty());
        let names: Vec<_> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"greet"));
        assert!(names.contains(&"Person"));
    }

    #[test]
    fn test_parse_rust() {
        let content = r#"
fn hello(name: &str) -> String {
    format!("Hello, {}!", name)
}

struct Person {
    name: String,
}

impl Person {
    fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}
"#;
        let path = PathBuf::from("test.rs");
        let symbols = find_all_symbols(&path, content);

        assert!(!symbols.is_empty());
        let names: Vec<_> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"hello"));
        assert!(names.contains(&"Person"));
    }

    #[test]
    fn test_get_symbol_at_position() {
        let content = "function greet(name) { return name; }";
        let path = PathBuf::from("test.js");

        // Position on "greet"
        let symbol = get_symbol_at_position(&path, content, 0, 9);
        assert!(symbol.is_some());
        let symbol = symbol.unwrap();
        assert_eq!(symbol.name, "greet");
    }
}
