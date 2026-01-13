//! Plugin permission system
//!
//! Validates and enforces plugin permissions declared in manifests.

use super::PluginManifest;
use glob::Pattern;
use std::path::Path;

/// Permission checker
///
/// Validates plugin operations against declared permissions.
pub struct PermissionChecker {
    manifests: Vec<PluginManifest>,
}

impl PermissionChecker {
    /// Create a new permission checker
    pub fn new() -> Self {
        Self {
            manifests: Vec::new(),
        }
    }

    /// Register a plugin's permissions
    pub fn register_plugin(&mut self, manifest: PluginManifest) {
        self.manifests.push(manifest);
    }

    /// Check if a plugin can read a file
    pub fn can_read_file(&self, plugin_name: &str, path: &Path) -> bool {
        self.check_file_permission(plugin_name, path, |manifest| &manifest.permissions.fs_read)
    }

    /// Check if a plugin can write a file
    pub fn can_write_file(&self, plugin_name: &str, path: &Path) -> bool {
        self.check_file_permission(plugin_name, path, |manifest| &manifest.permissions.fs_write)
    }

    /// Check if a plugin can access a network endpoint
    pub fn can_access_network(&self, plugin_name: &str, url: &str) -> bool {
        let manifest = match self.get_manifest(plugin_name) {
            Some(m) => m,
            None => return false,
        };

        // Check against network permissions
        for pattern in &manifest.permissions.network {
            if Self::matches_url_pattern(url, pattern) {
                return true;
            }
        }

        false
    }

    /// Check if a plugin can invoke a Tauri command
    pub fn can_invoke_command(&self, plugin_name: &str, command: &str) -> bool {
        let manifest = match self.get_manifest(plugin_name) {
            Some(m) => m,
            None => return false,
        };

        manifest.permissions.tauri_commands.contains(&command.to_string())
    }

    /// Check if a plugin can execute an external program
    pub fn can_execute_program(&self, plugin_name: &str, program: &str) -> bool {
        let manifest = match self.get_manifest(plugin_name) {
            Some(m) => m,
            None => return false,
        };

        manifest.permissions.external_programs.contains(&program.to_string())
    }

    /// Validate all permissions in a manifest
    pub fn validate_manifest(&self, manifest: &PluginManifest) -> Result<(), String> {
        // Validate file patterns
        for pattern in &manifest.permissions.fs_read {
            self.validate_file_pattern(pattern)?;
        }
        for pattern in &manifest.permissions.fs_write {
            self.validate_file_pattern(pattern)?;
        }

        // Validate network patterns
        for pattern in &manifest.permissions.network {
            self.validate_network_pattern(pattern)?;
        }

        Ok(())
    }

    /// Get a plugin's manifest
    fn get_manifest(&self, plugin_name: &str) -> Option<&PluginManifest> {
        self.manifests.iter().find(|m| m.plugin.name == plugin_name)
    }

    /// Check file permission using a getter function
    fn check_file_permission<F>(&self, plugin_name: &str, path: &Path, get_patterns: F) -> bool
    where
        F: Fn(&PluginManifest) -> &Vec<String>,
    {
        let manifest = match self.get_manifest(plugin_name) {
            Some(m) => m,
            None => return false,
        };

        let path_str = path.to_string_lossy();
        let patterns = get_patterns(manifest);

        for pattern in patterns {
            // Expand special variables
            let expanded = pattern
                .replace("$REPO", "**") // For now, allow any repo path
                .replace("$DATA", "**") // For now, allow any data path
                .replace("$HOME", "**"); // For now, allow any home path

            // Try to match as glob pattern
            if let Ok(glob_pattern) = Pattern::new(&expanded) {
                if glob_pattern.matches(&path_str) {
                    return true;
                }
            }

            // Also try exact match
            if pattern.as_str() == path_str {
                return true;
            }
        }

        false
    }

    /// Validate a file permission pattern
    fn validate_file_pattern(&self, pattern: &str) -> Result<(), String> {
        // Check for obviously dangerous patterns
        if pattern.contains("..") {
            return Err(format!("Invalid file pattern (path traversal): {}", pattern));
        }

        // Validate as glob pattern
        Pattern::new(pattern)
            .map_err(|e| format!("Invalid glob pattern '{}': {}", pattern, e))?;

        Ok(())
    }

    /// Validate a network permission pattern
    fn validate_network_pattern(&self, pattern: &str) -> Result<(), String> {
        // Must be a valid URL-like pattern
        if !pattern.starts_with("http://") && !pattern.starts_with("https://") {
            return Err(format!("Network pattern must start with http:// or https://: {}", pattern));
        }

        Ok(())
    }

    /// Match a URL against a pattern
    fn matches_url_pattern(url: &str, pattern: &str) -> bool {
        // Simple prefix matching for now
        // Could be enhanced with glob patterns

        if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            url.starts_with(prefix)
        } else {
            url == pattern
        }
    }
}

impl Default for PermissionChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::{PluginInfo, BinaryInfo, CompatibilityInfo, PluginPermissions};
    use std::path::PathBuf;

    fn create_test_manifest() -> PluginManifest {
        PluginManifest {
            plugin: PluginInfo {
                name: "test-plugin".to_string(),
                version: "0.1.0".to_string(),
                description: None,
                author: None,
                license: None,
            },
            binary: BinaryInfo {
                path: "libtest.dylib".to_string(),
            },
            compatibility: CompatibilityInfo {
                min_stage_version: None,
                max_stage_version: None,
                api_version: "0.1.0".to_string(),
            },
            permissions: PluginPermissions {
                fs_read: vec!["$REPO/**".to_string()],
                fs_write: vec!["$DATA/**".to_string()],
                network: vec!["https://api.example.com/*".to_string()],
                tauri_commands: vec!["get_file_diff".to_string()],
                external_programs: vec!["git".to_string()],
            },
            config: serde_json::Value::Null,
            frontend: None,
            hooks: Default::default(),
        }
    }

    #[test]
    fn test_file_permissions() {
        let mut checker = PermissionChecker::new();
        let manifest = create_test_manifest();
        checker.register_plugin(manifest);

        assert!(checker.can_read_file("test-plugin", &PathBuf::from("/repo/file.txt")));
        assert!(checker.can_write_file("test-plugin", &PathBuf::from("/data/file.txt")));
    }

    #[test]
    fn test_network_permissions() {
        let mut checker = PermissionChecker::new();
        let manifest = create_test_manifest();
        checker.register_plugin(manifest);

        assert!(checker.can_access_network("test-plugin", "https://api.example.com/endpoint"));
        assert!(!checker.can_access_network("test-plugin", "https://other.com/"));
    }

    #[test]
    fn test_command_permissions() {
        let mut checker = PermissionChecker::new();
        let manifest = create_test_manifest();
        checker.register_plugin(manifest);

        assert!(checker.can_invoke_command("test-plugin", "get_file_diff"));
        assert!(!checker.can_invoke_command("test-plugin", "delete_everything"));
    }

    #[test]
    fn test_program_permissions() {
        let mut checker = PermissionChecker::new();
        let manifest = create_test_manifest();
        checker.register_plugin(manifest);

        assert!(checker.can_execute_program("test-plugin", "git"));
        assert!(!checker.can_execute_program("test-plugin", "rm"));
    }

    #[test]
    fn test_invalid_patterns() {
        let checker = PermissionChecker::new();

        assert!(checker.validate_file_pattern("$REPO/**").is_ok());
        assert!(checker.validate_file_pattern("../etc/passwd").is_err());
        assert!(checker.validate_network_pattern("https://example.com").is_ok());
        assert!(checker.validate_network_pattern("ftp://example.com").is_err());
    }
}
