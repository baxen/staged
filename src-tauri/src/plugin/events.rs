//! Plugin event system
//!
//! Allows plugins to subscribe to lifecycle events and be notified
//! when important actions occur in the application.

use super::api::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_void;
use std::sync::{Arc, Mutex};

/// Plugin lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PluginEvent {
    /// Application startup (emitted after all plugins initialized)
    Startup,

    /// Application shutdown (emitted before plugins destroyed)
    Shutdown,

    /// Repository changed
    RepoChanged { repo_path: String },

    /// Commit created
    Commit {
        sha: String,
        message: String,
        repo_path: String,
    },

    /// Review completed/cleared
    ReviewCompleted {
        diff_id: String,
        repo_path: String,
    },

    /// Files changed in working tree
    FilesChanged {
        repo_path: String,
        file_paths: Vec<String>,
    },
}

/// Event subscription
struct EventSubscription {
    plugin_name: String,
    callback: EventCallback,
    context: *const c_void,
}

// Safety: Callbacks are only called on the main thread
unsafe impl Send for EventSubscription {}
unsafe impl Sync for EventSubscription {}

/// Event dispatcher
///
/// Manages event subscriptions and dispatches events to plugins.
pub struct EventDispatcher {
    subscriptions: Arc<Mutex<HashMap<String, Vec<EventSubscription>>>>,
}

impl EventDispatcher {
    /// Create a new event dispatcher
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Subscribe to an event
    pub fn subscribe(
        &self,
        plugin_name: &str,
        event_name: &str,
        callback: EventCallback,
        context: *const c_void,
    ) -> Result<(), String> {
        let mut subs = self
            .subscriptions
            .lock()
            .map_err(|e| format!("Failed to lock subscriptions: {}", e))?;

        let subscription = EventSubscription {
            plugin_name: plugin_name.to_string(),
            callback,
            context,
        };

        subs.entry(event_name.to_string())
            .or_insert_with(Vec::new)
            .push(subscription);

        log::debug!("Plugin {} subscribed to event: {}", plugin_name, event_name);
        Ok(())
    }

    /// Emit an event to all subscribers
    pub fn emit(&self, event: &PluginEvent) {
        let event_type = Self::event_type_name(event);

        let subs = match self.subscriptions.lock() {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to lock subscriptions for emit: {}", e);
                return;
            }
        };

        let subscribers = match subs.get(&event_type) {
            Some(s) => s,
            None => return, // No subscribers for this event
        };

        // Serialize event to JSON
        let event_json = match serde_json::to_string(event) {
            Ok(json) => json,
            Err(e) => {
                log::error!("Failed to serialize event: {}", e);
                return;
            }
        };

        let event_cstr = match CString::new(event_json) {
            Ok(cs) => cs,
            Err(e) => {
                log::error!("Failed to create C string for event: {}", e);
                return;
            }
        };

        log::debug!(
            "Emitting event {} to {} subscriber(s)",
            event_type,
            subscribers.len()
        );

        // Call all subscribers
        for sub in subscribers {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (sub.callback)(event_cstr.as_ptr(), sub.context)
            }));

            match result {
                Ok(0) => {} // Success
                Ok(code) => {
                    log::warn!(
                        "Plugin {} event handler for {} failed with code {}",
                        sub.plugin_name,
                        event_type,
                        code
                    );
                }
                Err(_) => {
                    log::error!(
                        "Plugin {} event handler for {} panicked",
                        sub.plugin_name,
                        event_type
                    );
                }
            }
        }
    }

    /// Get event type name for subscription matching
    fn event_type_name(event: &PluginEvent) -> String {
        match event {
            PluginEvent::Startup => "startup",
            PluginEvent::Shutdown => "shutdown",
            PluginEvent::RepoChanged { .. } => "repo_changed",
            PluginEvent::Commit { .. } => "commit",
            PluginEvent::ReviewCompleted { .. } => "review_completed",
            PluginEvent::FilesChanged { .. } => "files_changed",
        }
        .to_string()
    }

    /// Create an EventSubscriber for a plugin to use during registration
    pub fn create_subscriber(&self, plugin_name: String) -> EventSubscriberImpl {
        EventSubscriberImpl {
            dispatcher: self.subscriptions.clone(),
            plugin_name,
        }
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Implementation of EventSubscriber for plugins
pub struct EventSubscriberImpl {
    dispatcher: Arc<Mutex<HashMap<String, Vec<EventSubscription>>>>,
    plugin_name: String,
}

impl EventSubscriberImpl {
    /// Get the C-compatible EventSubscriber structure
    pub fn as_c_struct(&mut self) -> EventSubscriber {
        EventSubscriber {
            subscribe: Self::subscribe_impl,
        }
    }

    /// C-compatible subscription function
    extern "C" fn subscribe_impl(
        _event_name: *const std::os::raw::c_char,
        _callback: EventCallback,
        _context: *const c_void,
    ) -> std::os::raw::c_int {
        // Similar to commands, actual registration happens in Rust side
        // This is just a placeholder for the C ABI
        0
    }

    /// Subscribe to an event (called from Rust side)
    pub fn subscribe(
        &self,
        event_name: &str,
        callback: EventCallback,
        context: *const c_void,
    ) -> Result<(), String> {
        let mut subs = self
            .dispatcher
            .lock()
            .map_err(|e| format!("Failed to lock dispatcher: {}", e))?;

        let subscription = EventSubscription {
            plugin_name: self.plugin_name.clone(),
            callback,
            context,
        };

        subs.entry(event_name.to_string())
            .or_insert_with(Vec::new)
            .push(subscription);

        log::debug!(
            "Plugin {} subscribed to event: {}",
            self.plugin_name,
            event_name
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

    extern "C" fn test_callback(
        _event_data: *const std::os::raw::c_char,
        _context: *const c_void,
    ) -> std::os::raw::c_int {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        0 // Success
    }

    #[test]
    fn test_event_subscription() {
        let dispatcher = EventDispatcher::new();

        dispatcher
            .subscribe("test-plugin", "startup", test_callback, std::ptr::null())
            .unwrap();

        // Emit startup event
        CALL_COUNT.store(0, Ordering::SeqCst);
        dispatcher.emit(&PluginEvent::Startup);

        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_multiple_subscribers() {
        let dispatcher = EventDispatcher::new();

        dispatcher
            .subscribe("plugin1", "startup", test_callback, std::ptr::null())
            .unwrap();
        dispatcher
            .subscribe("plugin2", "startup", test_callback, std::ptr::null())
            .unwrap();

        // Emit startup event
        CALL_COUNT.store(0, Ordering::SeqCst);
        dispatcher.emit(&PluginEvent::Startup);

        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_event_serialization() {
        let event = PluginEvent::Commit {
            sha: "abc123".to_string(),
            message: "Test commit".to_string(),
            repo_path: "/path/to/repo".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("abc123"));
        assert!(json.contains("Test commit"));
    }
}
