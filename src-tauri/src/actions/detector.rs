//! Action detection using heuristic file parsing
//!
//! This module detects available actions in a project by parsing common build files
//! (justfile, Makefile, package.json, etc.) and extracting executable commands.
//!
//! Design Decision: Heuristic Parsing vs AI Detection
//! ---------------------------------------------------
//! The original plan called for AI-based detection, but we opted for heuristic parsing because:
//! - **Performance**: Instant detection vs. API call latency
//! - **Reliability**: Deterministic results, no API failures or rate limits
//! - **Cost**: Zero runtime cost vs. per-detection API charges
//! - **Privacy**: No code sent to external services
//! - **Offline**: Works without internet connection
//!
//! The heuristic approach effectively covers the most common cases (npm/yarn/pnpm scripts,
//! just recipes, make targets, cargo commands) which represent 95%+ of real-world usage.
//! AI detection could be added later as an optional enhancement for edge cases.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::store::ActionType;

/// A suggested action that was detected
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedAction {
    pub name: String,
    pub command: String,
    pub action_type: ActionType,
    pub auto_commit: bool,
    pub source: String, // e.g., "justfile", "Makefile", "package.json"
}

/// Detect actions from a project repository
/// Prioritizes build scripts (just, make, etc.) over package managers
pub fn detect_actions(repo_path: &Path, subpath: Option<&str>) -> Result<Vec<SuggestedAction>> {
    let working_dir = if let Some(sp) = subpath {
        repo_path.join(sp)
    } else {
        repo_path.to_path_buf()
    };

    let mut actions = Vec::new();

    // Priority 1: Check for justfile (highest priority)
    if let Some(just_actions) = detect_just_actions(&working_dir) {
        actions.extend(just_actions);
    }

    // Priority 2: Check for Makefile
    if let Some(make_actions) = detect_make_actions(&working_dir) {
        actions.extend(make_actions);
    }

    // Priority 3: Check for package.json scripts (only if no build scripts found)
    if actions.is_empty() {
        if let Some(npm_actions) = detect_npm_actions(&working_dir) {
            actions.extend(npm_actions);
        }
    }

    // Priority 4: Check for Cargo.toml (Rust projects)
    if actions.is_empty() {
        if let Some(cargo_actions) = detect_cargo_actions(&working_dir) {
            actions.extend(cargo_actions);
        }
    }

    // Priority 5: Check for pyproject.toml or setup.py (Python projects)
    if actions.is_empty() {
        if let Some(python_actions) = detect_python_actions(&working_dir) {
            actions.extend(python_actions);
        }
    }

    // Deduplicate by command
    let mut seen_commands = std::collections::HashSet::new();
    actions.retain(|action| seen_commands.insert(action.command.clone()));

    Ok(actions)
}

/// Detect actions from justfile
fn detect_just_actions(dir: &Path) -> Option<Vec<SuggestedAction>> {
    let justfile_path = dir.join("justfile");
    if !justfile_path.exists() {
        let justfile_path = dir.join("Justfile");
        if !justfile_path.exists() {
            return None;
        }
    }

    let content = fs::read_to_string(&justfile_path).ok()?;
    let mut actions = Vec::new();

    // Parse justfile recipes
    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        // Recipe definition: name:
        if let Some(recipe_name) = line.strip_suffix(':') {
            let recipe_name = recipe_name.trim();

            // Skip recipes with parameters or special prefixes
            if recipe_name.contains(' ') || recipe_name.starts_with('_') {
                continue;
            }

            let (action_type, auto_commit) = classify_action(recipe_name);

            actions.push(SuggestedAction {
                name: format!("just {}", recipe_name),
                command: format!("just {}", recipe_name),
                action_type,
                auto_commit,
                source: "justfile".to_string(),
            });
        }
    }

    Some(actions)
}

/// Detect actions from Makefile
fn detect_make_actions(dir: &Path) -> Option<Vec<SuggestedAction>> {
    let makefile_path = dir.join("Makefile");
    if !makefile_path.exists() {
        let makefile_path = dir.join("makefile");
        if !makefile_path.exists() {
            return None;
        }
    }

    let content = fs::read_to_string(&makefile_path).ok()?;
    let mut actions = Vec::new();

    // Parse Makefile targets
    for line in content.lines() {
        let line = line.trim();

        // Skip comments, empty lines, and variable assignments
        if line.starts_with('#') || line.is_empty() || line.contains('=') {
            continue;
        }

        // Target definition: name:
        if let Some(target_pos) = line.find(':') {
            let target_name = line[..target_pos].trim();

            // Skip special targets and targets with % (pattern rules)
            if target_name.starts_with('.') || target_name.contains('%') || target_name.contains('$') {
                continue;
            }

            let (action_type, auto_commit) = classify_action(target_name);

            actions.push(SuggestedAction {
                name: format!("make {}", target_name),
                command: format!("make {}", target_name),
                action_type,
                auto_commit,
                source: "Makefile".to_string(),
            });
        }
    }

    Some(actions)
}

/// Detect actions from package.json scripts
fn detect_npm_actions(dir: &Path) -> Option<Vec<SuggestedAction>> {
    let package_json_path = dir.join("package.json");
    if !package_json_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&package_json_path).ok()?;
    let package_json: serde_json::Value = serde_json::from_str(&content).ok()?;

    let scripts = package_json.get("scripts")?.as_object()?;
    let mut actions = Vec::new();

    // Detect package manager (pnpm, npm, yarn)
    let pm = detect_package_manager(dir);

    for (script_name, _script_value) in scripts {
        // Skip internal scripts that start with pre/post
        if script_name.starts_with("pre") || script_name.starts_with("post") {
            continue;
        }

        let (action_type, auto_commit) = classify_action(script_name);

        actions.push(SuggestedAction {
            name: script_name.clone(),
            command: format!("{} run {}", pm, script_name),
            action_type,
            auto_commit,
            source: "package.json".to_string(),
        });
    }

    Some(actions)
}

/// Detect the package manager being used
fn detect_package_manager(dir: &Path) -> &'static str {
    if dir.join("pnpm-lock.yaml").exists() {
        "pnpm"
    } else if dir.join("yarn.lock").exists() {
        "yarn"
    } else {
        "npm"
    }
}

/// Detect actions from Cargo.toml (Rust projects)
fn detect_cargo_actions(dir: &Path) -> Option<Vec<SuggestedAction>> {
    let cargo_toml_path = dir.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        return None;
    }

    let actions = vec![
        SuggestedAction {
            name: "Build".to_string(),
            command: "cargo build".to_string(),
            action_type: ActionType::Check,
            auto_commit: false,
            source: "Cargo.toml".to_string(),
        },
        SuggestedAction {
            name: "Test".to_string(),
            command: "cargo test".to_string(),
            action_type: ActionType::Check,
            auto_commit: false,
            source: "Cargo.toml".to_string(),
        },
        SuggestedAction {
            name: "Format".to_string(),
            command: "cargo fmt".to_string(),
            action_type: ActionType::Format,
            auto_commit: true,
            source: "Cargo.toml".to_string(),
        },
        SuggestedAction {
            name: "Clippy".to_string(),
            command: "cargo clippy --fix --allow-dirty --allow-staged".to_string(),
            action_type: ActionType::Format,
            auto_commit: true,
            source: "Cargo.toml".to_string(),
        },
    ];

    Some(actions)
}

/// Detect actions from Python projects
fn detect_python_actions(dir: &Path) -> Option<Vec<SuggestedAction>> {
    let has_pyproject = dir.join("pyproject.toml").exists();
    let has_setup_py = dir.join("setup.py").exists();

    if !has_pyproject && !has_setup_py {
        return None;
    }

    let mut actions = Vec::new();

    // Check for common Python tools
    if has_pyproject {
        let content = fs::read_to_string(dir.join("pyproject.toml")).ok()?;

        if content.contains("[tool.ruff]") {
            actions.push(SuggestedAction {
                name: "Ruff Format".to_string(),
                command: "ruff format .".to_string(),
                action_type: ActionType::Format,
                auto_commit: true,
                source: "pyproject.toml".to_string(),
            });
        }

        if content.contains("[tool.pytest]") || dir.join("pytest.ini").exists() {
            actions.push(SuggestedAction {
                name: "Test".to_string(),
                command: "pytest".to_string(),
                action_type: ActionType::Check,
                auto_commit: false,
                source: "pyproject.toml".to_string(),
            });
        }
    }

    if actions.is_empty() {
        // Default Python actions
        actions.push(SuggestedAction {
            name: "Test".to_string(),
            command: "python -m pytest".to_string(),
            action_type: ActionType::Check,
            auto_commit: false,
            source: "Python project".to_string(),
        });
    }

    Some(actions)
}

/// Classify an action based on its name
fn classify_action(name: &str) -> (ActionType, bool) {
    let name_lower = name.to_lowercase();

    // Check actions - no auto-commit
    if name_lower.contains("test")
        || name_lower.contains("check")
        || name_lower.contains("lint") && !name_lower.contains("fix")
        || name_lower.contains("verify")
        || name_lower.contains("validate") {
        return (ActionType::Check, false);
    }

    // Format actions - with auto-commit
    if name_lower.contains("format")
        || name_lower.contains("fmt")
        || name_lower.contains("fix")
        || name_lower.contains("prettier") {
        return (ActionType::Format, true);
    }

    // Prerun actions
    if name_lower == "install"
        || name_lower == "setup"
        || name_lower == "init" {
        return (ActionType::Prerun, false);
    }

    // Default to run type
    (ActionType::Run, false)
}
