//! Definition finding logic.
//!
//! Given a symbol, searches for its definition in:
//! 1. The same file
//! 2. Imported modules

use super::languages::SupportedLanguage;
use super::parser::{find_all_symbols, parse_source, SymbolInfo, SymbolKind};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tree_sitter::Node;

/// Result of a definition search.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionResult {
    /// Path to the file containing the definition
    pub file_path: String,
    /// Line number (0-indexed)
    pub line: usize,
    /// Column number (0-indexed)
    pub column: usize,
    /// End column number (0-indexed)
    pub end_column: usize,
    /// The symbol name
    pub name: String,
    /// The kind of symbol
    pub kind: SymbolKind,
    /// Preview of the definition line
    pub preview: String,
}

/// Find the definition of a symbol.
///
/// # Arguments
/// * `symbol` - The symbol to find
/// * `current_file` - Path to the file where the symbol was referenced
/// * `current_content` - Content of the current file
/// * `repo_root` - Root of the repository (for resolving imports)
/// * `read_file` - Function to read file contents
/// * `list_files` - Function to list files in a directory
///
/// # Returns
/// `Some(DefinitionResult)` if found, `None` otherwise.
pub fn find_definition<F, L>(
    symbol: &SymbolInfo,
    current_file: &Path,
    current_content: &str,
    repo_root: &Path,
    read_file: F,
    list_files: L,
) -> Option<DefinitionResult>
where
    F: Fn(&Path) -> Option<String>,
    L: Fn(&str) -> Vec<String>,
{
    let language = SupportedLanguage::from_path(current_file)?;

    // 1. Search in the same file first
    if let Some(def) = find_definition_in_file(&symbol.name, current_content, current_file) {
        // Don't return if it's the same location (we're already on the definition)
        if def.line != symbol.line || def.column != symbol.column {
            return Some(def);
        }
    }

    // 2. Try to resolve through imports
    let imports = extract_imports(current_content, language, current_file);
    for import in imports {
        if import.names.contains(&symbol.name) || import.names.contains(&"*".to_string()) {
            // Resolve the import path
            if let Some(resolved_path) = resolve_import_path(&import.source, current_file, repo_root)
            {
                if let Some(content) = read_file(&resolved_path) {
                    if let Some(def) =
                        find_definition_in_file(&symbol.name, &content, &resolved_path)
                    {
                        return Some(def);
                    }
                }
            }
        }
    }

    // 3. Try common patterns (index files, etc.)
    if let Some(def) = try_common_patterns(symbol, current_file, repo_root, &read_file, &list_files) {
        return Some(def);
    }

    None
}

/// Find a definition within a single file.
fn find_definition_in_file(name: &str, content: &str, file_path: &Path) -> Option<DefinitionResult> {
    let symbols = find_all_symbols(file_path, content);

    log::debug!(
        "find_definition_in_file: looking for '{}' in {:?}, found {} symbols",
        name,
        file_path.file_name(),
        symbols.len()
    );

    // Log first few symbols for debugging
    for (i, sym) in symbols.iter().take(10).enumerate() {
        log::debug!("  Symbol {}: {} ({:?})", i, sym.name, sym.kind);
    }

    // Find the first definition matching this name
    for symbol in symbols {
        if symbol.name == name && !matches!(symbol.kind, SymbolKind::Import | SymbolKind::Unknown) {
            log::debug!("Found definition: {} at line {}", symbol.name, symbol.line);
            return Some(DefinitionResult {
                file_path: file_path.to_string_lossy().to_string(),
                line: symbol.line,
                column: symbol.column,
                end_column: symbol.end_column,
                name: symbol.name,
                kind: symbol.kind,
                preview: symbol.context.unwrap_or_default(),
            });
        }
    }

    None
}

/// Import information extracted from source code.
#[derive(Debug)]
struct ImportInfo {
    /// The source/path of the import
    source: String,
    /// Names imported (e.g., ["foo", "bar"] for "import { foo, bar } from './module'")
    names: Vec<String>,
}

/// Extract imports from source code.
fn extract_imports(
    content: &str,
    language: SupportedLanguage,
    _file_path: &Path,
) -> Vec<ImportInfo> {
    let Some(tree) = parse_source(content, language) else {
        return vec![];
    };

    let mut imports = Vec::new();
    extract_imports_recursive(tree.root_node(), content, language, &mut imports);
    imports
}

/// Recursively extract import statements.
fn extract_imports_recursive(
    node: Node,
    content: &str,
    language: SupportedLanguage,
    imports: &mut Vec<ImportInfo>,
) {
    let kind = node.kind();

    match language {
        SupportedLanguage::TypeScript
        | SupportedLanguage::Tsx
        | SupportedLanguage::JavaScript
        | SupportedLanguage::Jsx => {
            if kind == "import_statement" {
                if let Some(import) = extract_js_import(node, content) {
                    imports.push(import);
                }
            }
        }
        SupportedLanguage::Python => {
            if kind == "import_statement" || kind == "import_from_statement" {
                if let Some(import) = extract_python_import(node, content) {
                    imports.push(import);
                }
            }
        }
        SupportedLanguage::Rust => {
            if kind == "use_declaration" {
                if let Some(import) = extract_rust_use(node, content) {
                    imports.push(import);
                }
            }
        }
        SupportedLanguage::Go => {
            if kind == "import_declaration" {
                if let Some(import) = extract_go_import(node, content) {
                    imports.push(import);
                }
            }
        }
        _ => {}
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_imports_recursive(child, content, language, imports);
    }
}

/// Extract import info from a JS/TS import statement.
fn extract_js_import(node: Node, content: &str) -> Option<ImportInfo> {
    let mut source = String::new();
    let mut names = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "string" | "string_fragment" => {
                // Remove quotes
                let text = child.utf8_text(content.as_bytes()).ok()?;
                source = text.trim_matches(|c| c == '"' || c == '\'').to_string();
            }
            "import_clause" => {
                // Extract named imports
                let mut inner_cursor = child.walk();
                for import_child in child.children(&mut inner_cursor) {
                    match import_child.kind() {
                        "identifier" => {
                            // Default import
                            if let Ok(name) = import_child.utf8_text(content.as_bytes()) {
                                names.push(name.to_string());
                            }
                        }
                        "named_imports" => {
                            // { foo, bar }
                            extract_named_imports(import_child, content, &mut names);
                        }
                        "namespace_import" => {
                            // * as foo
                            names.push("*".to_string());
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    if source.is_empty() {
        return None;
    }

    Some(ImportInfo { source, names })
}

/// Extract names from named imports: { foo, bar, baz as qux }
fn extract_named_imports(node: Node, content: &str, names: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "import_specifier" {
            // Get the imported name (could be renamed with "as")
            if let Some(name_node) = child.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(content.as_bytes()) {
                    names.push(name.to_string());
                }
            } else {
                // No "as" clause, use first identifier
                let mut inner_cursor = child.walk();
                for inner_child in child.children(&mut inner_cursor) {
                    if inner_child.kind() == "identifier" {
                        if let Ok(name) = inner_child.utf8_text(content.as_bytes()) {
                            names.push(name.to_string());
                            break;
                        }
                    }
                }
            }
        }
    }
}

/// Extract import info from a Python import statement.
fn extract_python_import(node: Node, content: &str) -> Option<ImportInfo> {
    let mut source = String::new();
    let mut names = Vec::new();

    if node.kind() == "import_statement" {
        // import foo, bar
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "dotted_name" {
                if let Ok(name) = child.utf8_text(content.as_bytes()) {
                    source = name.to_string();
                    names.push(name.split('.').last().unwrap_or(name).to_string());
                }
            }
        }
    } else if node.kind() == "import_from_statement" {
        // from foo import bar, baz
        if let Some(module) = node.child_by_field_name("module_name") {
            source = module.utf8_text(content.as_bytes()).ok()?.to_string();
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "dotted_name" || child.kind() == "identifier" {
                if let Ok(name) = child.utf8_text(content.as_bytes()) {
                    if name != &source {
                        names.push(name.to_string());
                    }
                }
            } else if child.kind() == "aliased_import" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Ok(name) = name_node.utf8_text(content.as_bytes()) {
                        names.push(name.to_string());
                    }
                }
            }
        }
    }

    if source.is_empty() {
        return None;
    }

    Some(ImportInfo { source, names })
}

/// Extract import info from a Rust use statement.
fn extract_rust_use(node: Node, content: &str) -> Option<ImportInfo> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some((path, names)) = extract_rust_use_path(child, content) {
            return Some(ImportInfo {
                source: path,
                names,
            });
        }
    }
    None
}

/// Extract path and names from a Rust use tree.
fn extract_rust_use_path(node: Node, content: &str) -> Option<(String, Vec<String>)> {
    match node.kind() {
        "scoped_identifier" | "identifier" => {
            let path = node.utf8_text(content.as_bytes()).ok()?.to_string();
            let name = path.split("::").last().unwrap_or(&path).to_string();
            Some((path, vec![name]))
        }
        "use_list" => {
            let mut names = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some((_, mut child_names)) = extract_rust_use_path(child, content) {
                    names.append(&mut child_names);
                }
            }
            Some((String::new(), names))
        }
        "scoped_use_list" => {
            let path = node
                .child_by_field_name("path")
                .and_then(|n| n.utf8_text(content.as_bytes()).ok())
                .map(|s| s.to_string())
                .unwrap_or_default();

            let mut names = Vec::new();
            if let Some(list) = node.child_by_field_name("list") {
                let mut cursor = list.walk();
                for child in list.children(&mut cursor) {
                    if let Some((_, mut child_names)) = extract_rust_use_path(child, content) {
                        names.append(&mut child_names);
                    }
                }
            }
            Some((path, names))
        }
        _ => None,
    }
}

/// Extract import info from a Go import statement.
fn extract_go_import(node: Node, content: &str) -> Option<ImportInfo> {
    let mut imports = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "import_spec_list" {
            let mut inner_cursor = child.walk();
            for spec in child.children(&mut inner_cursor) {
                if spec.kind() == "import_spec" {
                    if let Some(path) = spec.child_by_field_name("path") {
                        if let Ok(path_str) = path.utf8_text(content.as_bytes()) {
                            let clean_path = path_str.trim_matches('"');
                            let name = clean_path.split('/').last().unwrap_or(clean_path);
                            imports.push(ImportInfo {
                                source: clean_path.to_string(),
                                names: vec![name.to_string()],
                            });
                        }
                    }
                }
            }
        } else if child.kind() == "import_spec" {
            if let Some(path) = child.child_by_field_name("path") {
                if let Ok(path_str) = path.utf8_text(content.as_bytes()) {
                    let clean_path = path_str.trim_matches('"');
                    let name = clean_path.split('/').last().unwrap_or(clean_path);
                    return Some(ImportInfo {
                        source: clean_path.to_string(),
                        names: vec![name.to_string()],
                    });
                }
            }
        }
    }

    imports.into_iter().next()
}

/// Resolve an import path to an actual file path.
fn resolve_import_path(
    import_source: &str,
    current_file: &Path,
    repo_root: &Path,
) -> Option<PathBuf> {
    let current_dir = current_file.parent()?;
    let ext = current_file.extension()?.to_str()?;

    // Handle Rust module paths
    if ext == "rs" {
        return resolve_rust_module_path(import_source, current_file, repo_root);
    }

    // Handle JavaScript/TypeScript relative imports
    if import_source.starts_with('.') {
        let extensions = [
            "",
            ".ts",
            ".tsx",
            ".js",
            ".jsx",
            "/index.ts",
            "/index.tsx",
            "/index.js",
        ];

        for ext in extensions {
            let candidate = current_dir.join(format!("{}{}", import_source, ext));
            if candidate.exists() {
                return Some(candidate);
            }
        }
        return None;
    }

    // Handle absolute imports (from node_modules or src)
    // Try src/ directory first (common convention)
    let extensions = [".ts", ".tsx", ".js", ".jsx", "/index.ts", "/index.tsx"];
    for ext in extensions {
        let candidate = repo_root.join("src").join(format!("{}{}", import_source, ext));
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // Could also check node_modules, but that's usually too deep
    None
}

/// Resolve a Rust module path (e.g., "super::parser" or "crate::symbols::parser")
fn resolve_rust_module_path(
    module_path: &str,
    current_file: &Path,
    repo_root: &Path,
) -> Option<PathBuf> {
    let current_dir = current_file.parent()?;
    let parts: Vec<&str> = module_path.split("::").collect();

    if parts.is_empty() {
        return None;
    }

    let mut path_parts = Vec::new();
    let mut start_dir = current_dir.to_path_buf();

    for (i, part) in parts.iter().enumerate() {
        match *part {
            "super" => {
                // Go up one directory
                start_dir = start_dir.parent()?.to_path_buf();
            }
            "self" => {
                // Stay in current directory
            }
            "crate" => {
                // Go to crate root (find Cargo.toml)
                start_dir = find_crate_root(current_file, repo_root)?;
                // For lib.rs crate, src/ is the root
                if start_dir.join("src").exists() {
                    start_dir = start_dir.join("src");
                }
            }
            _ => {
                // This is a module name, collect remaining parts
                path_parts = parts[i..].to_vec();
                break;
            }
        }
    }

    if path_parts.is_empty() {
        return None;
    }

    // Try different module file patterns
    // 1. module_name.rs
    // 2. module_name/mod.rs
    // 3. For nested paths like parser::SymbolInfo, try parser.rs

    // Build the path from collected parts (excluding the last item which is the symbol name)
    let module_parts = if path_parts.len() > 1 {
        &path_parts[..path_parts.len() - 1]
    } else {
        &path_parts[..]
    };

    for (idx, _) in module_parts.iter().enumerate() {
        let subpath: PathBuf = module_parts[..=idx].iter().collect();

        // Try module_name.rs
        let candidate = start_dir.join(format!("{}.rs", subpath.display()));
        if candidate.exists() {
            return Some(candidate);
        }

        // Try module_name/mod.rs
        let candidate = start_dir.join(&subpath).join("mod.rs");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // Also try just the first part as a file
    let first_module = path_parts.first()?;
    let candidate = start_dir.join(format!("{}.rs", first_module));
    if candidate.exists() {
        return Some(candidate);
    }

    let candidate = start_dir.join(first_module).join("mod.rs");
    if candidate.exists() {
        return Some(candidate);
    }

    None
}

/// Find the crate root (directory containing Cargo.toml)
fn find_crate_root(current_file: &Path, repo_root: &Path) -> Option<PathBuf> {
    let mut dir = current_file.parent()?;

    while dir.starts_with(repo_root) {
        if dir.join("Cargo.toml").exists() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }

    None
}

/// Try common patterns to find definitions.
fn try_common_patterns<F, L>(
    symbol: &SymbolInfo,
    current_file: &Path,
    repo_root: &Path,
    read_file: &F,
    list_files: &L,
) -> Option<DefinitionResult>
where
    F: Fn(&Path) -> Option<String>,
    L: Fn(&str) -> Vec<String>,
{
    let language = SupportedLanguage::from_path(current_file)?;

    // Get current file's relative path from repo root for comparison
    let current_rel_path = current_file
        .strip_prefix(repo_root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // Get directory path relative to repo root
    let dir_rel_path = current_file
        .parent()
        .and_then(|p| p.strip_prefix(repo_root).ok())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    match language {
        SupportedLanguage::TypeScript
        | SupportedLanguage::Tsx
        | SupportedLanguage::JavaScript
        | SupportedLanguage::Jsx => {
            // Check for a file with the same name as the symbol
            let candidates = [
                format!("src/lib/{}.ts", symbol.name),
                format!("src/lib/{}.tsx", symbol.name),
                format!("src/components/{}.tsx", symbol.name),
                format!("src/components/{}.svelte", symbol.name),
                format!("src/{}.ts", symbol.name),
                format!("lib/{}.ts", symbol.name),
            ];

            for candidate in candidates {
                let path = repo_root.join(&candidate);
                if let Some(content) = read_file(&path) {
                    if let Some(def) = find_definition_in_file(&symbol.name, &content, &path) {
                        return Some(def);
                    }
                }
            }
        }
        SupportedLanguage::Rust => {
            log::debug!(
                "try_common_patterns: searching Rust sibling files in '{}'",
                dir_rel_path
            );

            // Get all files in the current directory via git
            let files = list_files(&dir_rel_path);

            for file_path in files {
                // Check if it's a .rs file and not the current file
                if file_path.ends_with(".rs") && file_path != current_rel_path {
                    log::debug!("  Checking sibling file: {}", file_path);
                    let full_path = repo_root.join(&file_path);
                    if let Some(content) = read_file(&full_path) {
                        log::debug!(
                            "    File read successfully, content length: {}",
                            content.len()
                        );
                        if let Some(def) =
                            find_definition_in_file(&symbol.name, &content, &full_path)
                        {
                            return Some(def);
                        }
                    } else {
                        log::debug!("    Failed to read file via git");
                    }
                }
            }

            // Check parent directory for lib.rs or main.rs
            if let Some(parent_dir) = Path::new(&dir_rel_path).parent() {
                let parent_dir_str = parent_dir.to_string_lossy().to_string();
                let parent_files = list_files(&parent_dir_str);

                for name in ["lib.rs", "main.rs"] {
                    let target = if parent_dir_str.is_empty() {
                        name.to_string()
                    } else {
                        format!("{}/{}", parent_dir_str, name)
                    };

                    if parent_files.iter().any(|f| f == &target || f.ends_with(&format!("/{}", name))) {
                        let full_path = repo_root.join(&target);
                        if let Some(content) = read_file(&full_path) {
                            if let Some(def) =
                                find_definition_in_file(&symbol.name, &content, &full_path)
                            {
                                return Some(def);
                            }
                        }
                    }
                }
            }
        }
        SupportedLanguage::Python => {
            // Get all files in the current directory
            let files = list_files(&dir_rel_path);

            // Check __init__.py first
            let init_path = if dir_rel_path.is_empty() {
                "__init__.py".to_string()
            } else {
                format!("{}/__init__.py", dir_rel_path)
            };

            if files.iter().any(|f| f == &init_path) && init_path != current_rel_path {
                let full_path = repo_root.join(&init_path);
                if let Some(content) = read_file(&full_path) {
                    if let Some(def) = find_definition_in_file(&symbol.name, &content, &full_path) {
                        return Some(def);
                    }
                }
            }

            // Search other .py files in the directory
            for file_path in files {
                if file_path.ends_with(".py") && file_path != current_rel_path {
                    let full_path = repo_root.join(&file_path);
                    if let Some(content) = read_file(&full_path) {
                        if let Some(def) =
                            find_definition_in_file(&symbol.name, &content, &full_path)
                        {
                            return Some(def);
                        }
                    }
                }
            }
        }
        SupportedLanguage::Go => {
            // Get all files in the current directory
            let files = list_files(&dir_rel_path);

            // Search all .go files in the same package (directory)
            for file_path in files {
                if file_path.ends_with(".go") && file_path != current_rel_path {
                    let full_path = repo_root.join(&file_path);
                    if let Some(content) = read_file(&full_path) {
                        if let Some(def) =
                            find_definition_in_file(&symbol.name, &content, &full_path)
                        {
                            return Some(def);
                        }
                    }
                }
            }
        }
        _ => {}
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_definition_in_same_file() {
        let content = r#"
function greet(name: string): string {
    return `Hello, ${name}!`;
}

const result = greet("World");
"#;
        let path = PathBuf::from("test.ts");
        let def = find_definition_in_file("greet", content, &path);

        assert!(def.is_some());
        let def = def.unwrap();
        assert_eq!(def.name, "greet");
        assert_eq!(def.line, 1);
    }

    #[test]
    fn test_extract_js_imports() {
        let content = r#"
import { foo, bar } from './module';
import defaultExport from './other';
import * as all from './all';
"#;
        let imports = extract_imports(
            content,
            SupportedLanguage::TypeScript,
            Path::new("test.ts"),
        );

        assert!(!imports.is_empty());
        assert!(imports.iter().any(|i| i.names.contains(&"foo".to_string())));
        assert!(imports.iter().any(|i| i.names.contains(&"bar".to_string())));
    }
}
