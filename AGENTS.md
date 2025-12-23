# AGENTS.md

## Overview

**Staged** is a desktop git diff viewer. Tauri app with Rust backend (git2 for git ops) and Svelte/TypeScript frontend.

## Architecture

```
Frontend (Svelte/TS)           Backend (Rust/Tauri)
────────────────────           ────────────────────
src/                           src-tauri/src/
  App.svelte                     lib.rs (Tauri commands)
  lib/                           git.rs (git2 operations)
    *.svelte (components)
    services/git.ts ──invoke──►
    types.ts
```

Frontend calls Rust via Tauri's `invoke()`. All git operations happen in Rust using libgit2.

## Commands

Use `just` for all dev tasks:

```bash
just dev        # Run with hot-reload
just fmt        # Format all code (cargo fmt + prettier)
just lint       # Clippy for Rust
just typecheck  # Type check everything
just check-all  # All checks before submitting
```

## Code Quality

**Always format and lint your work before finishing:**
```bash
just fmt        # Auto-format Rust + TypeScript/Svelte
just check-all  # Verify everything passes
```

- Rust: `cargo fmt` + `cargo clippy`
- TypeScript/Svelte: `prettier`

## Git Workflow

**Do not** create branches, commit, or push unless explicitly asked. The human manages git operations.
