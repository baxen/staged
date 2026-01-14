//! Plugin menu registration
//!
//! Allows plugins to add menu items to the application menu bar.

use super::api::*;
use std::collections::HashMap;
use std::os::raw::{c_char, c_void};
use std::sync::{Arc, Mutex};

/// A registered menu item
#[derive(Debug, Clone)]
pub struct PluginMenuItem {
    pub plugin_name: String,
    pub id: String,
    pub parent: String,
    pub label: String,
    pub shortcut: Option<String>,
}

/// Menu registry
///
/// Stores menu items registered by plugins. The actual menu creation
/// happens in the Tauri setup, but plugins can query this registry.
pub struct MenuRegistry {
    items: Arc<Mutex<HashMap<String, PluginMenuItem>>>,
}

impl MenuRegistry {
    /// Create a new menu registry
    pub fn new() -> Self {
        Self {
            items: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a menu item
    pub fn register_item(
        &self,
        plugin_name: &str,
        id: &str,
        parent: &str,
        label: &str,
        shortcut: Option<&str>,
    ) -> Result<(), String> {
        let full_id = format!("{}:{}", plugin_name, id);

        let mut items = self
            .items
            .lock()
            .map_err(|e| format!("Failed to lock menu registry: {}", e))?;

        if items.contains_key(&full_id) {
            return Err(format!("Menu item {} already registered", full_id));
        }

        let item = PluginMenuItem {
            plugin_name: plugin_name.to_string(),
            id: id.to_string(),
            parent: parent.to_string(),
            label: label.to_string(),
            shortcut: shortcut.map(|s| s.to_string()),
        };

        items.insert(full_id.clone(), item);

        log::info!("Registered menu item: {}", full_id);
        Ok(())
    }

    /// Get all registered menu items
    pub fn get_items(&self) -> Result<Vec<PluginMenuItem>, String> {
        let items = self
            .items
            .lock()
            .map_err(|e| format!("Failed to lock menu registry: {}", e))?;

        Ok(items.values().cloned().collect())
    }

    /// Get menu items for a specific parent menu
    pub fn get_items_for_parent(&self, parent: &str) -> Result<Vec<PluginMenuItem>, String> {
        let items = self
            .items
            .lock()
            .map_err(|e| format!("Failed to lock menu registry: {}", e))?;

        Ok(items
            .values()
            .filter(|item| item.parent == parent)
            .cloned()
            .collect())
    }

    /// Create a MenuRegistrar for a plugin to use during registration
    pub fn create_registrar(&self, plugin_name: String) -> MenuRegistrarImpl {
        MenuRegistrarImpl {
            registry: self.items.clone(),
            plugin_name,
        }
    }
}

impl Default for MenuRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Implementation of MenuRegistrar for plugins
pub struct MenuRegistrarImpl {
    registry: Arc<Mutex<HashMap<String, PluginMenuItem>>>,
    plugin_name: String,
}

impl MenuRegistrarImpl {
    /// Get the C-compatible MenuRegistrar structure
    pub fn as_c_struct(&mut self) -> MenuRegistrar {
        MenuRegistrar {
            add_menu_item: Self::add_menu_item_impl,
        }
    }

    /// C-compatible menu item registration function
    extern "C" fn add_menu_item_impl(
        _parent: *const c_char,
        _id: *const c_char,
        _label: *const c_char,
        _shortcut: *const c_char,
        _context: *const c_void,
    ) -> std::os::raw::c_int {
        // Actual registration happens in Rust side
        0
    }

    /// Register a menu item (called from Rust side)
    pub fn register(
        &self,
        id: &str,
        parent: &str,
        label: &str,
        shortcut: Option<&str>,
    ) -> Result<(), String> {
        let full_id = format!("{}:{}", self.plugin_name, id);

        let mut items = self
            .registry
            .lock()
            .map_err(|e| format!("Failed to lock registry: {}", e))?;

        if items.contains_key(&full_id) {
            return Err(format!("Menu item {} already registered", full_id));
        }

        let item = PluginMenuItem {
            plugin_name: self.plugin_name.clone(),
            id: id.to_string(),
            parent: parent.to_string(),
            label: label.to_string(),
            shortcut: shortcut.map(|s| s.to_string()),
        };

        items.insert(full_id.clone(), item);

        log::debug!("Registered menu item: {}", full_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_registration() {
        let registry = MenuRegistry::new();

        registry
            .register_item(
                "test-plugin",
                "my-command",
                "Tools",
                "My Command",
                Some("CmdOrCtrl+Shift+T"),
            )
            .unwrap();

        let items = registry.get_items().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "My Command");
    }

    #[test]
    fn test_duplicate_menu_item() {
        let registry = MenuRegistry::new();

        registry
            .register_item("test-plugin", "my-command", "Tools", "My Command", None)
            .unwrap();

        let result = registry.register_item(
            "test-plugin",
            "my-command",
            "Tools",
            "My Command",
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_menu_items_by_parent() {
        let registry = MenuRegistry::new();

        registry
            .register_item("plugin1", "cmd1", "Tools", "Command 1", None)
            .unwrap();
        registry
            .register_item("plugin2", "cmd2", "File", "Command 2", None)
            .unwrap();
        registry
            .register_item("plugin3", "cmd3", "Tools", "Command 3", None)
            .unwrap();

        let tools_items = registry.get_items_for_parent("Tools").unwrap();
        assert_eq!(tools_items.len(), 2);
    }
}
