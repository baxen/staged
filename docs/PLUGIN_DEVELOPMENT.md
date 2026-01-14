# Stage Plugin Development Guide

This guide explains how to create plugins for Stage, the diff viewer and code review tool.

## Table of Contents

- [Overview](#overview)
- [Plugin Architecture](#plugin-architecture)
- [Quick Start: Simple Plugin](#quick-start-simple-plugin)
- [Plugin API Reference](#plugin-api-reference)
- [Building and Installing Plugins](#building-and-installing-plugins)
- [Advanced Examples](#advanced-examples)

---

## Overview

Stage supports a plugin system that allows you to extend functionality through:
- **Backend commands** (Rust/C) - Execute system commands, integrate with external tools
- **Frontend UI** (JavaScript) - Display modals, interact with Stage's state
- **Keyboard shortcuts** - Register custom shortcuts for your plugin
- **Menu items** - Add items to Stage's menu bar

Plugins can be:
- **Frontend-only** - Just JavaScript/CSS (e.g., a theme plugin)
- **Backend-only** - Just Rust commands (e.g., a linter integration)
- **Full-stack** - Both backend and frontend (e.g., Builder Bot integration)

---

## Plugin Architecture

### Directory Structure

```
my-plugin/
├── plugin.toml           # Plugin manifest
├── src/
│   └── lib.rs           # Backend code (optional)
├── frontend/
│   └── main.js          # Frontend code (optional)
│   └── styles.css       # Styles (optional)
├── dist/                # Built frontend assets
│   └── my-plugin.js
│   └── my-plugin.css
├── Cargo.toml           # Rust dependencies (if backend)
└── package.json         # JS dependencies (if frontend)
```

### Plugin Manifest (plugin.toml)

Every plugin needs a `plugin.toml` file:

```toml
[plugin]
name = "my-plugin"
version = "0.1.0"
description = "A simple example plugin"

[plugin.frontend]
script = "dist/my-plugin.js"
style = "dist/my-plugin.css"  # optional

[plugin.backend]
library = "libmy_plugin.dylib"  # .so on Linux, .dll on Windows

[plugin.permissions]
read_files = ["**/*.txt"]      # File read permissions (globs)
write_files = []               # File write permissions
network = ["https://api.example.com/*"]  # Network access
commands = ["git", "npm"]      # External commands to execute
```

---

## Quick Start: Simple Plugin

Let's create the simplest possible plugin - a frontend-only plugin that shows a modal.

### Step 1: Create Plugin Directory

```bash
mkdir hello-stage-plugin
cd hello-stage-plugin
```

### Step 2: Create plugin.toml

```toml
[plugin]
name = "hello-stage"
version = "0.1.0"
description = "A simple hello world plugin"

[plugin.frontend]
script = "dist/hello-stage.js"
```

### Step 3: Create Frontend Code

Create `frontend/main.js`:

```javascript
/**
 * Hello Stage Plugin
 *
 * This is the simplest possible Stage plugin.
 * It registers a modal that displays "Hello, Stage!"
 */

// All plugins register themselves on window.StagedPlugins
window.StagedPlugins = window.StagedPlugins || {};

// Register your plugin with its name from plugin.toml
window.StagedPlugins['hello-stage'] = function(api) {
  console.log('[Hello Stage] Plugin initializing');

  // Define the modal component
  // This is a plain function that takes (target, props) and returns cleanup function
  function HelloModal(target, props) {
    console.log('[Hello Stage] Modal opened');

    // Create the UI
    const container = document.createElement('div');
    container.style.cssText = `
      padding: 40px;
      background: var(--bg-chrome, #1e1e1e);
      color: var(--text-primary, #e0e0e0);
      border-radius: 8px;
      text-align: center;
      font-family: system-ui, sans-serif;
    `;

    const title = document.createElement('h1');
    title.textContent = '👋 Hello, Stage!';
    title.style.cssText = 'font-size: 32px; margin: 0 0 16px 0;';

    const message = document.createElement('p');
    message.textContent = 'This is your first Stage plugin!';
    message.style.cssText = 'font-size: 16px; margin: 0 0 24px 0;';

    const closeButton = document.createElement('button');
    closeButton.textContent = 'Close';
    closeButton.style.cssText = `
      padding: 10px 24px;
      background: var(--accent-color, #0078d4);
      color: white;
      border: none;
      border-radius: 6px;
      font-size: 14px;
      cursor: pointer;
    `;
    closeButton.onclick = () => props.api.closeModal();

    container.appendChild(title);
    container.appendChild(message);
    container.appendChild(closeButton);
    target.appendChild(container);

    // Return cleanup function
    return function cleanup() {
      console.log('[Hello Stage] Modal closed');
      container.remove();
    };
  }

  // Register the modal with the plugin API
  api.registerModal('main', HelloModal);

  console.log('[Hello Stage] Plugin initialized');
};
```

### Step 4: Build the Plugin

Create `package.json`:

```json
{
  "name": "hello-stage-plugin",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "build": "vite build"
  },
  "devDependencies": {
    "vite": "^5.0.0"
  }
}
```

Create `vite.config.js`:

```javascript
import { defineConfig } from 'vite';

export default defineConfig({
  build: {
    lib: {
      entry: 'frontend/main.js',
      name: 'HelloStagePlugin',
      formats: ['iife'],
      fileName: () => 'hello-stage.js'
    },
    outDir: 'dist'
  }
});
```

Build it:

```bash
npm install
npm run build
```

### Step 5: Install the Plugin

Copy your plugin to Stage's plugin directory:

```bash
# macOS
cp -r . ~/Library/Application\ Support/staged/plugins/hello-stage/

# Linux
cp -r . ~/.config/staged/plugins/hello-stage/

# Windows
cp -r . %APPDATA%\staged\plugins\hello-stage\
```

### Step 6: Test It

1. Restart Stage
2. Open the plugin modal: `Cmd+Shift+H` or via menu
3. You should see your "Hello, Stage!" modal!

---

## Plugin API Reference

When your plugin initializes, it receives an `api` object with these methods:

### `api.invoke(command, payload)`

Call a backend command registered by your plugin.

```javascript
// Call the 'execute' command
const result = await api.invoke('execute', {
  prompt: 'Do something',
  repo_path: '/path/to/repo'
});
console.log(result);
```

### `api.registerModal(id, component)`

Register a modal component that can be opened.

```javascript
function MyModal(target, props) {
  // Create UI
  const div = document.createElement('div');
  target.appendChild(div);

  // Return cleanup
  return () => div.remove();
}

api.registerModal('main', MyModal);
```

### `api.getCurrentRepo()`

Get the currently open repository path.

```javascript
const repoPath = await api.getCurrentRepo();
console.log('Current repo:', repoPath);
```

### `api.getCurrentDiff()`

Get the current diff specification.

```javascript
const diffSpec = await api.getCurrentDiff();
console.log('Current diff:', diffSpec);
// Example: { base: { type: "Rev", value: "HEAD" }, head: { type: "WorkingTree" } }
```

### `api.showModal(id, props)`

Open a modal registered by your plugin.

```javascript
api.showModal('main', {
  initialMessage: 'Hello from props!'
});
```

### `api.closeModal()`

Close the currently open modal.

```javascript
api.closeModal();
```

---

## Building and Installing Plugins

### Frontend-Only Plugin

1. **Create the plugin structure** (see Quick Start above)
2. **Build with Vite**:
   ```bash
   npm install
   npm run build
   ```
3. **Install**:
   ```bash
   # macOS
   mkdir -p ~/Library/Application\ Support/staged/plugins/your-plugin
   cp -r dist plugin.toml ~/Library/Application\ Support/staged/plugins/your-plugin/
   ```

### Full-Stack Plugin (Rust + JavaScript)

For a plugin with backend commands, you'll need Rust:

**src/lib.rs**:

```rust
use std::ffi::{CStr, CString, c_char, c_int, c_void};

// Stage Plugin API types
#[repr(C)]
pub struct PluginVTable {
    pub api_version: u32,
    pub init: extern "C" fn(context: *const PluginContext) -> c_int,
    pub shutdown: extern "C" fn() -> c_int,
    pub register_commands: extern "C" fn(registrar: *mut CommandRegistrar) -> c_int,
    pub register_menus: extern "C" fn(registrar: *mut MenuRegistrar) -> c_int,
    pub subscribe_events: extern "C" fn(subscriber: *mut EventSubscriber) -> c_int,
    pub get_frontend_bundle: Option<extern "C" fn(bundle: *mut FrontendBundle) -> c_int>,
}

#[repr(C)]
pub struct PluginContext {
    pub app_handle: *const c_void,
    pub data_dir: *const c_char,
    pub config: *const c_char,
}

#[repr(C)]
pub struct CommandRegistrar {
    pub register_fn: extern "C" fn(
        name: *const c_char,
        handler: CommandHandler,
        context: *const c_void,
    ) -> c_int,
}

pub type CommandHandler = extern "C" fn(
    request: *const c_char,
    response: *mut c_char,
    response_len: usize,
    context: *const c_void,
) -> c_int;

#[repr(C)]
pub struct MenuRegistrar {
    pub add_menu_item: extern "C" fn(
        parent: *const c_char,
        id: *const c_char,
        label: *const c_char,
        shortcut: *const c_char,
        context: *const c_void,
    ) -> c_int,
}

#[repr(C)]
pub struct EventSubscriber {
    pub subscribe: extern "C" fn(
        event_name: *const c_char,
        callback: EventCallback,
        context: *const c_void,
    ) -> c_int,
}

pub type EventCallback =
    extern "C" fn(event_data: *const c_char, context: *const c_void) -> c_int;

#[repr(C)]
pub struct FrontendBundle {
    pub js_path: *const c_char,
    pub css_path: *const c_char,
}

// Plugin lifecycle functions
extern "C" fn init(_context: *const PluginContext) -> c_int {
    println!("[My Plugin] Initialized");
    0 // Success
}

extern "C" fn shutdown() -> c_int {
    println!("[My Plugin] Shutdown");
    0
}

// Example command handler
extern "C" fn hello_command(
    _request: *const c_char,
    response: *mut c_char,
    response_len: usize,
    _context: *const c_void,
) -> c_int {
    unsafe {
        let message = r#"{"message":"Hello from plugin!"}"#;
        let bytes = message.as_bytes();

        if bytes.len() + 1 > response_len {
            return -1; // Buffer too small
        }

        std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, response, bytes.len());
        *response.add(bytes.len()) = 0; // null terminator

        0 // Success
    }
}

extern "C" fn register_commands(registrar: *mut CommandRegistrar) -> c_int {
    unsafe {
        let reg = &*registrar;
        let cmd_name = CString::new("hello").unwrap();
        (reg.register_fn)(cmd_name.as_ptr(), hello_command, std::ptr::null());
    }
    0
}

extern "C" fn register_menus(_registrar: *mut MenuRegistrar) -> c_int {
    0 // No menus
}

extern "C" fn subscribe_events(_subscriber: *mut EventSubscriber) -> c_int {
    0 // No events
}

extern "C" fn get_frontend_bundle(bundle: *mut FrontendBundle) -> c_int {
    unsafe {
        let js_path = CString::new("dist/my-plugin.js").unwrap();
        (*bundle).js_path = js_path.into_raw();
        0
    }
}

// Plugin entry point - Stage calls this to get the vtable
#[no_mangle]
pub extern "C" fn staged_plugin_entry() -> *const PluginVTable {
    static VTABLE: PluginVTable = PluginVTable {
        api_version: 0x000100, // 0.1.0
        init,
        shutdown,
        register_commands,
        register_menus,
        subscribe_events,
        get_frontend_bundle: Some(get_frontend_bundle),
    };

    &VTABLE as *const PluginVTable
}
```

**Cargo.toml**:

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**Build**:

```bash
# Build Rust backend
cargo build --release

# Build frontend
npm run build

# Copy to plugin directory (macOS)
mkdir -p ~/Library/Application\ Support/staged/plugins/my-plugin
cp target/release/libmy_plugin.dylib ~/Library/Application\ Support/staged/plugins/my-plugin/
cp -r dist plugin.toml ~/Library/Application\ Support/staged/plugins/my-plugin/
```

---

## Advanced Examples

### Accessing Props in Modal

```javascript
function MyModal(target, props) {
  console.log('Received props:', props);

  // Access initial data
  const message = props.initialMessage || 'Default message';

  // Access the plugin API
  const { api } = props;

  // Create UI with prop data
  const div = document.createElement('div');
  div.textContent = message;
  target.appendChild(div);

  return () => div.remove();
}
```

### Calling Backend Commands

```javascript
async function createTask() {
  try {
    const repo = await props.api.getCurrentRepo();
    const result = await props.api.invoke('create_task', {
      description: 'Fix the bug',
      repo_path: repo
    });

    if (result.success) {
      console.log('Task created:', result.task_id);
    }
  } catch (e) {
    console.error('Failed:', e);
  }
}
```

### Styling with CSS Variables

Stage provides CSS variables you can use:

```javascript
container.style.cssText = `
  background: var(--bg-chrome);      /* Main background */
  color: var(--text-primary);        /* Primary text */
  border: 1px solid var(--border-color);  /* Borders */
`;
```

Available variables:
- `--bg-chrome` - Main background color
- `--bg-secondary` - Secondary background
- `--bg-hover` - Hover state background
- `--text-primary` - Primary text color
- `--text-secondary` - Secondary text color
- `--text-muted` - Muted text color
- `--border-color` - Border color
- `--accent-color` - Accent/action color

---

## Plugin Installation Locations

### macOS
```
~/Library/Application Support/staged/plugins/
└── your-plugin/
    ├── plugin.toml
    ├── dist/
    └── libplugin.dylib (if backend)
```

### Linux
```
~/.config/staged/plugins/
└── your-plugin/
    ├── plugin.toml
    ├── dist/
    └── libplugin.so (if backend)
```

### Windows
```
%APPDATA%\staged\plugins\
└── your-plugin\
    ├── plugin.toml
    ├── dist\
    └── plugin.dll (if backend)
```

---

## Tips and Best Practices

1. **Use console.log liberally** during development - check the browser console for frontend logs
2. **Modal cleanup** - Always return a cleanup function to avoid memory leaks
3. **Error handling** - Wrap async operations in try/catch
4. **CSS variables** - Use Stage's CSS variables for consistent theming
5. **Plugin naming** - Use kebab-case for plugin names (e.g., `my-plugin`)
6. **Version your API calls** - Check `api_version` if you depend on specific features

---

## Troubleshooting

### Plugin not loading

Check Stage's logs for errors:
```bash
# macOS/Linux
tail -f ~/Library/Logs/staged/stage.log
```

Common issues:
- Missing `plugin.toml`
- Incorrect file paths in manifest
- JavaScript syntax errors (check browser console)
- Missing dependencies in Rust build

### Commands not found

Make sure:
1. Commands are registered in `register_commands`
2. Command names match what frontend calls
3. Backend library is built and copied to plugin directory

### Modal not appearing

Check:
1. Modal is registered: `api.registerModal('main', MyModal)`
2. Modal is being opened: `api.showModal('your-plugin:main')`
3. Browser console for JavaScript errors

---

## Examples in the Wild

- **builder-bot** - Full-stack plugin integrating Builder Bot CLI
- **test-plugin** - Simple frontend-only demo plugin

Check out the Stage repository for complete source code of these plugins.

---

## Need Help?

- [GitHub Issues](https://github.com/baxen/staged/issues)
- [Discussions](https://github.com/baxen/staged/discussions)

Happy plugin development! 🚀
