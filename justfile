# Staged - Git Diff Viewer

# Run the app in development mode
dev:
    npm run tauri:dev

# Build the app for production
build:
    npm run tauri:build

# Run just the frontend (for quick UI iteration)
frontend:
    npm run dev

# Check Rust code
check:
    cd src-tauri && cargo check

# Format Rust code
fmt:
    cd src-tauri && cargo fmt

# Run Svelte type checking
typecheck:
    npm run check

# Install dependencies
install:
    npm install
    cd src-tauri && cargo fetch

# Clean build artifacts
clean:
    rm -rf dist
    rm -rf src-tauri/target
    rm -rf node_modules
