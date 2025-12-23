# Staged

A standalone git diff viewer inspired by IntelliJ's diff UI. Built with Tauri (Rust backend) + Svelte (TypeScript frontend) for a fast, native desktop experience with side-by-side diffs and integrated staging.

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (1.77.2+)
- [Node.js](https://nodejs.org/) (v18+)
- [just](https://github.com/casey/just) (command runner)

### Quick Start

```bash
just install   # Install npm + cargo dependencies
just dev       # Run in development mode (hot-reload)
```

### Commands

```bash
just dev        # Run app in dev mode with hot-reload
just build      # Build for production
just frontend   # Run just the frontend (quick UI iteration)

# Code quality
just fmt        # Format all code (Rust + TypeScript/Svelte)
just lint       # Lint Rust with clippy
just typecheck  # Type check TypeScript + Svelte + Rust
just check-all  # Run all checks (format, lint, typecheck)

# Maintenance
just install    # Install all dependencies
just clean      # Remove build artifacts
```

## Features

- File tree with staged, unstaged, and untracked sections
- Side-by-side diff viewer with line numbers
- Stage/unstage individual files
- Discard changes with confirmation
- Current branch display
