//! Plugin command registry and dispatcher
//!
//! This module implements the runtime command dispatch system that allows
//! plugins to register commands callable from the frontend.

use super::api::*;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::{Arc, Mutex};

/// Maximum response buffer size for plugin commands (64KB)
const MAX_RESPONSE_SIZE: usize = 65536;

/// A registered plugin command
struct RegisteredCommand {
    /// Plugin name that owns this command
    plugin_name: String,
    /// Command handler function
    handler: CommandHandler,
    /// User context pointer passed to handler
    context: *const c_void,
}

// Safety: We ensure context pointers remain valid for the lifetime of the command
unsafe impl Send for RegisteredCommand {}
unsafe impl Sync for RegisteredCommand {}

/// Plugin command registry
///
/// Maintains a mapping of command names to their handlers and provides
/// a dispatcher for invoking them.
pub struct PluginCommandRegistry {
    commands: Arc<Mutex<HashMap<String, RegisteredCommand>>>,
}

impl PluginCommandRegistry {
    /// Create a new empty command registry
    pub fn new() -> Self {
        Self {
            commands: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a command from a plugin
    pub fn register_command(
        &self,
        plugin_name: &str,
        command_name: &str,
        handler: CommandHandler,
        context: *const c_void,
    ) -> Result<(), String> {
        let full_name = format!("{}:{}", plugin_name, command_name);

        let mut commands = self.commands.lock()
            .map_err(|e| format!("Failed to lock command registry: {}", e))?;

        if commands.contains_key(&full_name) {
            return Err(format!("Command {} already registered", full_name));
        }

        commands.insert(
            full_name.clone(),
            RegisteredCommand {
                plugin_name: plugin_name.to_string(),
                handler,
                context,
            },
        );

        log::info!("Registered plugin command: {}", full_name);
        Ok(())
    }

    /// Invoke a plugin command
    pub fn invoke_command(
        &self,
        plugin_name: &str,
        command_name: &str,
        payload: &str,
    ) -> Result<String, String> {
        let full_name = format!("{}:{}", plugin_name, command_name);

        // Look up command
        let commands = self.commands.lock()
            .map_err(|e| format!("Failed to lock command registry: {}", e))?;

        let cmd = commands.get(&full_name)
            .ok_or_else(|| format!("Command {} not found", full_name))?;

        // Call the command handler
        Self::call_handler(cmd, payload)
    }

    /// Call a command handler with JSON payload
    fn call_handler(cmd: &RegisteredCommand, payload: &str) -> Result<String, String> {
        // Prepare request C string
        let request_cstr = CString::new(payload)
            .map_err(|e| format!("Invalid payload: {}", e))?;

        // Allocate response buffer
        let mut response_buf = vec![0i8; MAX_RESPONSE_SIZE];

        // Call handler (wrapped in panic catch with AssertUnwindSafe)
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (cmd.handler)(
                request_cstr.as_ptr(),
                response_buf.as_mut_ptr(),
                response_buf.len(),
                cmd.context,
            )
        }));

        match result {
            Ok(code) => {
                if code != 0 {
                    return Err(format!("Command failed with code {}", code));
                }

                // Parse response
                let response = unsafe {
                    CStr::from_ptr(response_buf.as_ptr())
                        .to_str()
                        .map_err(|e| format!("Invalid UTF-8 in response: {}", e))?
                        .to_string()
                };

                Ok(response)
            }
            Err(_) => {
                Err(format!("Plugin command panicked"))
            }
        }
    }

    /// Get list of all registered commands
    pub fn list_commands(&self) -> Result<Vec<String>, String> {
        let commands = self.commands.lock()
            .map_err(|e| format!("Failed to lock command registry: {}", e))?;

        Ok(commands.keys().cloned().collect())
    }

    /// Get commands for a specific plugin
    pub fn list_plugin_commands(&self, plugin_name: &str) -> Result<Vec<String>, String> {
        let commands = self.commands.lock()
            .map_err(|e| format!("Failed to lock command registry: {}", e))?;

        let prefix = format!("{}:", plugin_name);
        let plugin_commands: Vec<String> = commands
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .map(|k| k.strip_prefix(&prefix).unwrap().to_string())
            .collect();

        Ok(plugin_commands)
    }

    /// Create a CommandRegistrar for a plugin to use during registration
    pub fn create_registrar(&self, plugin_name: String) -> CommandRegistrarImpl {
        CommandRegistrarImpl {
            registry: self.commands.clone(),
            plugin_name,
        }
    }
}

impl Default for PluginCommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Implementation of CommandRegistrar for plugins
///
/// This is what gets passed to plugins during the register_commands phase.
pub struct CommandRegistrarImpl {
    registry: Arc<Mutex<HashMap<String, RegisteredCommand>>>,
    plugin_name: String,
}

// Thread-local storage for the current registrar
thread_local! {
    static CURRENT_REGISTRAR: std::cell::RefCell<Option<Arc<Mutex<HashMap<String, RegisteredCommand>>>>> = std::cell::RefCell::new(None);
    static CURRENT_PLUGIN_NAME: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

impl CommandRegistrarImpl {
    /// Get the C-compatible CommandRegistrar structure
    pub fn as_c_struct(&mut self) -> CommandRegistrar {
        // Set this registrar as the current one for this thread
        CURRENT_REGISTRAR.with(|r| {
            *r.borrow_mut() = Some(self.registry.clone());
        });
        CURRENT_PLUGIN_NAME.with(|n| {
            *n.borrow_mut() = Some(self.plugin_name.clone());
        });

        CommandRegistrar {
            register_fn: Self::register_fn_impl,
        }
    }

    /// C-compatible registration function
    extern "C" fn register_fn_impl(
        name: *const c_char,
        handler: CommandHandler,
        context: *const c_void,
    ) -> c_int {
        unsafe {
            // Extract command name from C string
            let command_name = match CStr::from_ptr(name).to_str() {
                Ok(s) => s,
                Err(_) => {
                    log::error!("Invalid command name (not UTF-8)");
                    return -1;
                }
            };

            // Get the current registrar from TLS
            let result = CURRENT_REGISTRAR.with(|r| {
                let registry_opt = r.borrow();
                let registry = match registry_opt.as_ref() {
                    Some(reg) => reg,
                    None => {
                        log::error!("No registrar set in TLS");
                        return Err(-1);
                    }
                };

                CURRENT_PLUGIN_NAME.with(|n| {
                    let plugin_name_opt = n.borrow();
                    let plugin_name = match plugin_name_opt.as_ref() {
                        Some(name) => name,
                        None => {
                            log::error!("No plugin name set in TLS");
                            return Err(-1);
                        }
                    };

                    let full_name = format!("{}:{}", plugin_name, command_name);

                    let mut commands = match registry.lock() {
                        Ok(c) => c,
                        Err(e) => {
                            log::error!("Failed to lock command registry: {}", e);
                            return Err(-1);
                        }
                    };

                    if commands.contains_key(&full_name) {
                        log::warn!("Command {} already registered", full_name);
                        return Err(-1);
                    }

                    commands.insert(
                        full_name.clone(),
                        RegisteredCommand {
                            plugin_name: plugin_name.clone(),
                            handler,
                            context,
                        },
                    );

                    log::info!("Registered plugin command: {}", full_name);
                    Ok(0)
                })
            });

            result.unwrap_or(-1)
        }
    }

    /// Register a command (called from Rust side)
    pub fn register(
        &self,
        command_name: &str,
        handler: CommandHandler,
        context: *const c_void,
    ) -> Result<(), String> {
        let full_name = format!("{}:{}", self.plugin_name, command_name);

        let mut commands = self.registry.lock()
            .map_err(|e| format!("Failed to lock registry: {}", e))?;

        if commands.contains_key(&full_name) {
            return Err(format!("Command {} already registered", full_name));
        }

        commands.insert(
            full_name.clone(),
            RegisteredCommand {
                plugin_name: self.plugin_name.clone(),
                handler,
                context,
            },
        );

        log::debug!("Registered command: {}", full_name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test command handler that echoes the input
    extern "C" fn echo_handler(
        request: *const c_char,
        response: *mut c_char,
        response_len: usize,
        _context: *const c_void,
    ) -> c_int {
        unsafe {
            let req = CStr::from_ptr(request).to_str().unwrap();
            let resp = format!(r#"{{"echo":"{}"}}"#, req);

            if resp.len() + 1 > response_len {
                return -1; // Buffer too small
            }

            std::ptr::copy_nonoverlapping(
                resp.as_ptr() as *const c_char,
                response,
                resp.len(),
            );
            *response.add(resp.len()) = 0; // null terminator

            0 // Success
        }
    }

    #[test]
    fn test_command_registration() {
        let registry = PluginCommandRegistry::new();

        registry
            .register_command("test-plugin", "echo", echo_handler, std::ptr::null())
            .unwrap();

        let commands = registry.list_commands().unwrap();
        assert_eq!(commands, vec!["test-plugin:echo"]);
    }

    #[test]
    fn test_command_invocation() {
        let registry = PluginCommandRegistry::new();

        registry
            .register_command("test-plugin", "echo", echo_handler, std::ptr::null())
            .unwrap();

        let result = registry
            .invoke_command("test-plugin", "echo", "hello")
            .unwrap();

        assert_eq!(result, r#"{"echo":"hello"}"#);
    }

    #[test]
    fn test_duplicate_registration() {
        let registry = PluginCommandRegistry::new();

        registry
            .register_command("test-plugin", "echo", echo_handler, std::ptr::null())
            .unwrap();

        let result = registry.register_command("test-plugin", "echo", echo_handler, std::ptr::null());

        assert!(result.is_err());
    }
}
