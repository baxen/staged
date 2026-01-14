//! Plugin C ABI definitions
//!
//! This module defines the stable C ABI that plugins must implement.
//! Using C ABI ensures compatibility across Rust compiler versions.

use std::os::raw::{c_char, c_int, c_void};

/// Current plugin API version (0.1.0 = 0x000100)
pub const PLUGIN_API_VERSION: u32 = 0x000100;

/// Plugin VTable - the main interface that plugins expose
///
/// This is a C-compatible structure containing function pointers for all
/// plugin capabilities. Plugins export a function `staged_plugin_entry()`
/// that returns a pointer to this structure.
#[repr(C)]
pub struct PluginVTable {
    /// API version (must match PLUGIN_API_VERSION)
    pub api_version: u32,

    /// Initialize the plugin with context
    /// Returns 0 on success, non-zero on error
    pub init: extern "C" fn(context: *const PluginContext) -> c_int,

    /// Shutdown the plugin
    /// Returns 0 on success, non-zero on error
    pub shutdown: extern "C" fn() -> c_int,

    /// Register plugin commands
    /// Returns 0 on success, non-zero on error
    pub register_commands: extern "C" fn(registrar: *mut CommandRegistrar) -> c_int,

    /// Register menu items
    /// Returns 0 on success, non-zero on error
    pub register_menus: extern "C" fn(registrar: *mut MenuRegistrar) -> c_int,

    /// Subscribe to lifecycle events
    /// Returns 0 on success, non-zero on error
    pub subscribe_events: extern "C" fn(subscriber: *mut EventSubscriber) -> c_int,

    /// Get frontend assets (optional - can be null)
    /// Returns 0 on success, non-zero if no assets
    pub get_frontend_bundle: Option<extern "C" fn(bundle: *mut FrontendBundle) -> c_int>,
}

/// Plugin initialization context
///
/// Passed to the plugin's init() function. Contains pointers to app resources
/// and configuration data.
#[repr(C)]
pub struct PluginContext {
    /// Opaque pointer to Tauri AppHandle (for internal use)
    pub app_handle: *const c_void,

    /// Path to plugin's data directory (UTF-8 encoded)
    pub data_dir: *const c_char,

    /// Plugin configuration as JSON string (UTF-8 encoded)
    pub config: *const c_char,
}

/// Command registrar for adding new Tauri commands
///
/// Plugins use this to register command handlers that can be invoked
/// from the frontend.
#[repr(C)]
pub struct CommandRegistrar {
    /// Register a command handler
    ///
    /// Parameters:
    /// - name: Command name (UTF-8)
    /// - handler: Handler function pointer
    /// - context: User context pointer (passed to handler)
    ///
    /// Returns 0 on success, non-zero on error
    pub register_fn: extern "C" fn(
        name: *const c_char,
        handler: CommandHandler,
        context: *const c_void,
    ) -> c_int,
}

/// Command handler function signature
///
/// Takes JSON request, writes JSON response to buffer.
///
/// Parameters:
/// - request: JSON payload input (UTF-8)
/// - response: JSON response output buffer (UTF-8)
/// - response_len: Maximum response buffer length
/// - context: User context pointer
///
/// Returns 0 on success, non-zero on error.
pub type CommandHandler = extern "C" fn(
    request: *const c_char,
    response: *mut c_char,
    response_len: usize,
    context: *const c_void,
) -> c_int;

/// Menu registrar for adding menu items
#[repr(C)]
pub struct MenuRegistrar {
    /// Add a menu item
    ///
    /// Parameters:
    /// - parent: Parent menu name (UTF-8): "File", "Edit", "Tools", etc.
    /// - id: Menu item ID (UTF-8)
    /// - label: Menu item label (UTF-8)
    /// - shortcut: Keyboard shortcut (optional, UTF-8): "CmdOrCtrl+Shift+B"
    /// - context: User context pointer
    ///
    /// Returns 0 on success, non-zero on error
    pub add_menu_item: extern "C" fn(
        parent: *const c_char,
        id: *const c_char,
        label: *const c_char,
        shortcut: *const c_char,
        context: *const c_void,
    ) -> c_int,
}

/// Event subscriber for lifecycle hooks
#[repr(C)]
pub struct EventSubscriber {
    /// Subscribe to an event
    ///
    /// Parameters:
    /// - event_name: Event name (UTF-8): "startup", "commit", "review_completed", etc.
    /// - callback: Callback function
    /// - context: User context pointer
    ///
    /// Returns 0 on success, non-zero on error
    pub subscribe: extern "C" fn(
        event_name: *const c_char,
        callback: EventCallback,
        context: *const c_void,
    ) -> c_int,
}

/// Event callback function signature
///
/// Called when subscribed event occurs.
///
/// Parameters:
/// - event_data: Event data as JSON (UTF-8)
/// - context: User context pointer
///
/// Returns 0 on success, non-zero on error.
pub type EventCallback = extern "C" fn(
    event_data: *const c_char,
    context: *const c_void,
) -> c_int;

/// Frontend bundle info
///
/// Returned by get_frontend_bundle() to specify JavaScript/CSS assets.
#[repr(C)]
pub struct FrontendBundle {
    /// Path to JavaScript bundle (UTF-8, relative to plugin dir)
    pub js_path: *const c_char,

    /// Path to CSS bundle (UTF-8, relative to plugin dir, can be null)
    pub css_path: *const c_char,
}

/// Plugin entry point signature
///
/// Every plugin must export a function with this signature named `staged_plugin_entry`.
/// The function returns a pointer to the plugin's VTable.
pub type PluginEntryFn = extern "C" fn() -> *const PluginVTable;

/// Helper to safely read a C string into a Rust String
///
/// # Safety
/// The pointer must be valid and point to a null-terminated UTF-8 string.
pub unsafe fn c_str_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    std::ffi::CStr::from_ptr(ptr)
        .to_str()
        .ok()
        .map(|s| s.to_string())
}

/// Helper to write a Rust string to a C buffer
///
/// Returns false if the string is too long for the buffer.
///
/// # Safety
/// The buffer must be valid and have at least `buffer_len` bytes.
pub unsafe fn string_to_c_buffer(s: &str, buffer: *mut c_char, buffer_len: usize) -> bool {
    let bytes = s.as_bytes();

    // Need room for null terminator
    if bytes.len() + 1 > buffer_len {
        return false;
    }

    std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, buffer, bytes.len());
    *buffer.add(bytes.len()) = 0; // null terminator

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vtable_size() {
        // VTable should be a reasonable size
        let size = std::mem::size_of::<PluginVTable>();
        assert!(size < 1024, "VTable is unexpectedly large: {} bytes", size);
    }

    #[test]
    fn test_string_helpers() {
        unsafe {
            // Test string_to_c_buffer
            let mut buffer = vec![0i8; 100];
            let result = string_to_c_buffer("hello", buffer.as_mut_ptr(), buffer.len());
            assert!(result);

            // Test c_str_to_string
            let s = c_str_to_string(buffer.as_ptr());
            assert_eq!(s, Some("hello".to_string()));

            // Test buffer too small
            let mut small_buffer = vec![0i8; 3];
            let result = string_to_c_buffer("hello", small_buffer.as_mut_ptr(), small_buffer.len());
            assert!(!result);
        }
    }
}
