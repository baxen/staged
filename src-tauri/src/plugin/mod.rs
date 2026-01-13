//! Plugin system for Stage
//!
//! This module implements a dynamic plugin system that allows loading
//! third-party functionality at runtime. Plugins are distributed as compiled
//! binaries (.dylib/.so/.dll) with TOML manifests.

pub mod api;
pub mod commands;
pub mod events;
pub mod menus;

use api::*;
use commands::{PluginCommandRegistry, CommandRegistrarImpl};
use events::{EventDispatcher, EventSubscriberImpl};
use menus::{MenuRegistry, MenuRegistrarImpl};
use libloading::{Library, Symbol};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

// Re-export for use in lib.rs
pub use events::PluginEvent;

/// Plugin manifest structure (loaded from plugin.toml)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginManifest {
    pub plugin: PluginInfo,
    pub binary: BinaryInfo,
    pub compatibility: CompatibilityInfo,
    #[serde(default)]
    pub permissions: PluginPermissions,
    #[serde(default)]
    pub config: serde_json::Value,
    pub frontend: Option<FrontendInfo>,
    #[serde(default)]
    pub hooks: HooksInfo,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BinaryInfo {
    /// Relative path from plugin directory
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompatibilityInfo {
    pub min_stage_version: Option<String>,
    pub max_stage_version: Option<String>,
    pub api_version: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PluginPermissions {
    #[serde(default)]
    pub fs_read: Vec<String>,
    #[serde(default)]
    pub fs_write: Vec<String>,
    #[serde(default)]
    pub network: Vec<String>,
    #[serde(default)]
    pub tauri_commands: Vec<String>,
    #[serde(default)]
    pub external_programs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrontendInfo {
    pub script: String,
    pub style: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HooksInfo {
    #[serde(default)]
    pub startup: bool,
    #[serde(default)]
    pub commit: bool,
    #[serde(default)]
    pub review_completed: bool,
}

/// Plugin loading state
#[derive(Debug)]
enum PluginState {
    /// Successfully loaded and initialized
    Loaded,
    /// Initialization failed
    Failed(String),
}

/// A loaded plugin with its resources
struct LoadedPlugin {
    manifest: PluginManifest,
    #[allow(dead_code)] // Keep library alive while plugin is loaded
    library: Library,
    vtable: &'static PluginVTable,
    state: PluginState,
}

/// Plugin manager - orchestrates plugin discovery, loading, and lifecycle
pub struct PluginManager {
    plugins: HashMap<String, LoadedPlugin>,
    plugins_dir: PathBuf,
    command_registry: PluginCommandRegistry,
    event_dispatcher: EventDispatcher,
    menu_registry: MenuRegistry,
}

impl PluginManager {
    /// Create a new plugin manager
    ///
    /// Plugins are expected to be in ~/.config/staged/plugins/
    pub fn new() -> Result<Self, String> {
        let plugins_dir = dirs::config_dir()
            .ok_or_else(|| "Could not determine config directory".to_string())?
            .join("staged")
            .join("plugins");

        // Create plugins directory if it doesn't exist
        if !plugins_dir.exists() {
            std::fs::create_dir_all(&plugins_dir)
                .map_err(|e| format!("Failed to create plugins directory: {}", e))?;
        }

        Ok(Self {
            plugins: HashMap::new(),
            plugins_dir,
            command_registry: PluginCommandRegistry::new(),
            event_dispatcher: EventDispatcher::new(),
            menu_registry: MenuRegistry::new(),
        })
    }

    /// Discover all plugin manifests in the plugins directory
    pub fn discover_plugins(&self) -> Result<Vec<(PathBuf, PluginManifest)>, String> {
        let mut manifests = Vec::new();

        let entries = std::fs::read_dir(&self.plugins_dir)
            .map_err(|e| format!("Failed to read plugins directory: {}", e))?;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    log::warn!("Failed to read plugin directory entry: {}", e);
                    continue;
                }
            };

            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let manifest_path = path.join("plugin.toml");
            if !manifest_path.exists() {
                continue;
            }

            match Self::load_manifest(&manifest_path) {
                Ok(manifest) => {
                    log::info!("Discovered plugin: {}", manifest.plugin.name);
                    manifests.push((path, manifest));
                }
                Err(e) => {
                    log::warn!("Failed to load manifest at {:?}: {}", manifest_path, e);
                }
            }
        }

        Ok(manifests)
    }

    /// Load a plugin manifest from a TOML file
    fn load_manifest(path: &Path) -> Result<PluginManifest, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read manifest: {}", e))?;

        toml::from_str(&content)
            .map_err(|e| format!("Failed to parse manifest: {}", e))
    }

    /// Load a plugin from a manifest
    pub fn load_plugin(
        &mut self,
        plugin_dir: PathBuf,
        manifest: PluginManifest,
    ) -> Result<(), String> {
        let plugin_name = manifest.plugin.name.clone();

        // Check if already loaded
        if self.plugins.contains_key(&plugin_name) {
            return Err(format!("Plugin {} is already loaded", plugin_name));
        }

        // Validate compatibility
        self.validate_compatibility(&manifest)?;

        // Construct full path to binary
        let lib_path = plugin_dir.join(&manifest.binary.path);
        if !lib_path.exists() {
            return Err(format!(
                "Plugin binary not found at {:?}",
                lib_path
            ));
        }

        log::info!("Loading plugin {} from {:?}", plugin_name, lib_path);

        // Load the dynamic library
        let library = unsafe {
            Library::new(&lib_path)
                .map_err(|e| format!("Failed to load plugin library: {}", e))?
        };

        // Get the plugin entry point
        let entry: Symbol<PluginEntryFn> = unsafe {
            library
                .get(b"staged_plugin_entry")
                .map_err(|e| format!("Plugin missing entry point 'staged_plugin_entry': {}", e))?
        };

        // Call entry point to get VTable
        let vtable_ptr = entry();
        if vtable_ptr.is_null() {
            return Err("Plugin entry point returned null VTable".to_string());
        }

        let vtable = unsafe { &*vtable_ptr };

        // Verify API version
        if vtable.api_version != PLUGIN_API_VERSION {
            return Err(format!(
                "Plugin API version mismatch: expected 0x{:06X}, got 0x{:06X}",
                PLUGIN_API_VERSION, vtable.api_version
            ));
        }

        log::info!("Plugin {} loaded successfully", plugin_name);

        // Store the loaded plugin
        self.plugins.insert(
            plugin_name.clone(),
            LoadedPlugin {
                manifest,
                library,
                vtable,
                state: PluginState::Loaded,
            },
        );

        Ok(())
    }

    /// Initialize a loaded plugin
    pub fn initialize_plugin(
        &mut self,
        plugin_name: &str,
        app_handle: &AppHandle,
    ) -> Result<(), String> {
        // Get plugin data directory before borrowing plugin
        let data_dir = self.get_plugin_data_dir(plugin_name)?;
        let data_dir_cstr = CString::new(data_dir.to_string_lossy().as_bytes())
            .map_err(|e| format!("Invalid data dir path: {}", e))?;

        // Get plugin and serialize config
        let plugin = self.plugins.get_mut(plugin_name)
            .ok_or_else(|| format!("Plugin {} not found", plugin_name))?;

        let config_json = serde_json::to_string(&plugin.manifest.config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        let config_cstr = CString::new(config_json)
            .map_err(|e| format!("Invalid config JSON: {}", e))?;

        // Create plugin context
        let context = PluginContext {
            app_handle: app_handle as *const _ as *const std::os::raw::c_void,
            data_dir: data_dir_cstr.as_ptr(),
            config: config_cstr.as_ptr(),
        };

        // Call init
        log::info!("Initializing plugin {}", plugin_name);
        let result = (plugin.vtable.init)(&context);

        if result != 0 {
            let error_msg = format!("Plugin init failed with code {}", result);
            plugin.state = PluginState::Failed(error_msg.clone());
            return Err(error_msg);
        }

        plugin.state = PluginState::Loaded;
        log::info!("Plugin {} initialized successfully", plugin_name);

        Ok(())
    }

    /// Get the data directory for a plugin
    fn get_plugin_data_dir(&self, plugin_name: &str) -> Result<PathBuf, String> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| "Could not determine data directory".to_string())?
            .join("staged")
            .join("plugins")
            .join(plugin_name);

        // Create if doesn't exist
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)
                .map_err(|e| format!("Failed to create plugin data directory: {}", e))?;
        }

        Ok(data_dir)
    }

    /// Validate plugin compatibility with current Stage version
    fn validate_compatibility(&self, manifest: &PluginManifest) -> Result<(), String> {
        // Parse plugin's required API version
        let plugin_api_version = Self::parse_api_version(&manifest.compatibility.api_version)?;

        // Check if it matches our API version
        if plugin_api_version != PLUGIN_API_VERSION {
            return Err(format!(
                "Incompatible plugin API version: plugin requires 0x{:06X}, Stage provides 0x{:06X}",
                plugin_api_version, PLUGIN_API_VERSION
            ));
        }

        // TODO: Check min_stage_version and max_stage_version

        Ok(())
    }

    /// Parse API version string (e.g., "0.1.0") to u32 (0x000100)
    fn parse_api_version(version: &str) -> Result<u32, String> {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid API version format: {}", version));
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| format!("Invalid minor version: {}", parts[1]))?;
        let patch = parts[2]
            .parse::<u32>()
            .map_err(|_| format!("Invalid patch version: {}", parts[2]))?;

        Ok((major << 16) | (minor << 8) | patch)
    }

    /// Get a list of all loaded plugin names
    pub fn plugin_names(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Get a plugin's manifest
    pub fn get_manifest(&self, plugin_name: &str) -> Option<&PluginManifest> {
        self.plugins.get(plugin_name).map(|p| &p.manifest)
    }

    /// Register commands for a plugin
    pub fn register_plugin_commands(&mut self, plugin_name: &str) -> Result<(), String> {
        let plugin = self.plugins.get(plugin_name)
            .ok_or_else(|| format!("Plugin {} not found", plugin_name))?;

        log::info!("Registering commands for plugin {}", plugin_name);

        // Create a registrar for this plugin
        let mut registrar_impl = self.command_registry.create_registrar(plugin_name.to_string());
        let mut registrar = registrar_impl.as_c_struct();

        // Call the plugin's register_commands function
        let result = (plugin.vtable.register_commands)(&mut registrar as *mut _);

        if result != 0 {
            return Err(format!("Plugin command registration failed with code {}", result));
        }

        log::info!("Successfully registered commands for plugin {}", plugin_name);
        Ok(())
    }

    /// Invoke a plugin command
    pub fn invoke_command(
        &self,
        plugin_name: &str,
        command_name: &str,
        payload: &str,
    ) -> Result<String, String> {
        self.command_registry.invoke_command(plugin_name, command_name, payload)
    }

    /// Get all registered commands
    pub fn list_commands(&self) -> Result<Vec<String>, String> {
        self.command_registry.list_commands()
    }

    /// Register menu items for a plugin
    pub fn register_plugin_menus(&mut self, plugin_name: &str) -> Result<(), String> {
        let plugin = self.plugins.get(plugin_name)
            .ok_or_else(|| format!("Plugin {} not found", plugin_name))?;

        log::info!("Registering menus for plugin {}", plugin_name);

        let mut registrar_impl = self.menu_registry.create_registrar(plugin_name.to_string());
        let mut registrar = registrar_impl.as_c_struct();

        let result = (plugin.vtable.register_menus)(&mut registrar as *mut _);

        if result != 0 {
            return Err(format!("Plugin menu registration failed with code {}", result));
        }

        log::info!("Successfully registered menus for plugin {}", plugin_name);
        Ok(())
    }

    /// Subscribe plugin to events
    pub fn subscribe_plugin_events(&mut self, plugin_name: &str) -> Result<(), String> {
        let plugin = self.plugins.get(plugin_name)
            .ok_or_else(|| format!("Plugin {} not found", plugin_name))?;

        log::info!("Subscribing {} to events", plugin_name);

        let mut subscriber_impl = self.event_dispatcher.create_subscriber(plugin_name.to_string());
        let mut subscriber = subscriber_impl.as_c_struct();

        let result = (plugin.vtable.subscribe_events)(&mut subscriber as *mut _);

        if result != 0 {
            return Err(format!("Plugin event subscription failed with code {}", result));
        }

        log::info!("Successfully subscribed {} to events", plugin_name);
        Ok(())
    }

    /// Emit an event to all subscribed plugins
    pub fn emit_event(&self, event: &PluginEvent) {
        self.event_dispatcher.emit(event);
    }

    /// Get all registered menu items
    pub fn get_menu_items(&self) -> Result<Vec<menus::PluginMenuItem>, String> {
        self.menu_registry.get_items()
    }

    /// Shutdown all plugins
    pub fn shutdown_all(&mut self) {
        // Emit shutdown event
        self.emit_event(&PluginEvent::Shutdown);

        for (name, plugin) in &self.plugins {
            log::info!("Shutting down plugin {}", name);
            let result = (plugin.vtable.shutdown)();
            if result != 0 {
                log::warn!("Plugin {} shutdown failed with code {}", name, result);
            }
        }
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        self.shutdown_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_api_version() {
        assert_eq!(
            PluginManager::parse_api_version("0.1.0").unwrap(),
            0x000100
        );
        assert_eq!(
            PluginManager::parse_api_version("1.2.3").unwrap(),
            0x010203
        );
        assert!(PluginManager::parse_api_version("invalid").is_err());
        assert!(PluginManager::parse_api_version("1.2").is_err());
    }

    #[test]
    fn test_manifest_parsing() {
        let toml_str = r#"
            [plugin]
            name = "test-plugin"
            version = "0.1.0"
            description = "A test plugin"

            [binary]
            path = "libtest.dylib"

            [compatibility]
            api_version = "0.1.0"

            [permissions]
            fs_read = ["$REPO/**"]

            [config]
            test_key = "test_value"
        "#;

        let manifest: PluginManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.plugin.name, "test-plugin");
        assert_eq!(manifest.binary.path, "libtest.dylib");
        assert_eq!(manifest.compatibility.api_version, "0.1.0");
        assert_eq!(manifest.permissions.fs_read, vec!["$REPO/**"]);
    }
}
