//! CLI discovery for finding command-line tools.
//!
//! GUI apps on macOS don't inherit the shell's PATH, so we need to find CLIs
//! by running `which` through a login shell that sources the user's shell config.

use std::path::PathBuf;
use std::process::Command;

/// Common paths where CLIs might be installed.
const COMMON_PATHS: &[&str] = &[
    "/opt/homebrew/bin",              // Homebrew on Apple Silicon
    "/usr/local/bin",                 // Homebrew on Intel Mac
    "/usr/bin",                       // System binaries
    "/home/linuxbrew/.linuxbrew/bin", // Linuxbrew
];

/// Find a CLI command by name.
///
/// Tries multiple strategies:
/// 1. Login shell `which` (inherits user's full PATH from shell config)
/// 2. Common installation paths (fallback)
pub fn find_command(cmd: &str) -> Option<PathBuf> {
    // Strategy 1: Try login shell `which`
    if let Some(path) = find_via_login_shell(cmd) {
        if path.exists() {
            return Some(path);
        }
    }

    // Strategy 2: Check common installation paths
    for dir in COMMON_PATHS {
        let path = PathBuf::from(dir).join(cmd);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Find a command via login shell `which`.
fn find_via_login_shell(cmd: &str) -> Option<PathBuf> {
    let which_cmd = format!("which {}", cmd);

    // Try zsh first (default on macOS)
    if let Ok(output) = Command::new("/bin/zsh")
        .args(["-l", "-c", &which_cmd])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(path_str) = stdout.lines().rfind(|l| !l.is_empty()) {
                let path_str = path_str.trim();
                if !path_str.is_empty() && path_str.starts_with('/') {
                    return Some(PathBuf::from(path_str));
                }
            }
        }
    }

    // Fallback to bash
    if let Ok(output) = Command::new("/bin/bash")
        .args(["-l", "-c", &which_cmd])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(path_str) = stdout.lines().rfind(|l| !l.is_empty()) {
                let path_str = path_str.trim();
                if !path_str.is_empty() && path_str.starts_with('/') {
                    return Some(PathBuf::from(path_str));
                }
            }
        }
    }

    None
}
