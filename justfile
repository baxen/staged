# Staged - Git Diff Viewer

# Run the app in development mode (optionally point to another repo)
dev repo="":
    {{ if repo != "" { "STAGED_REPO=" + repo + " " } else { "" } }}npm run tauri:dev

# Build the app for production (unsigned)
build:
    npm run tauri:build

# Run just the frontend (for quick UI iteration)
frontend:
    npm run dev

# ============================================================================
# Code Quality
# ============================================================================

# Format all code (Rust + TypeScript/Svelte)
fmt:
    cd src-tauri && cargo fmt
    npx prettier --write "src/**/*.{ts,svelte,css,html}"

# Check formatting without modifying files
fmt-check:
    cd src-tauri && cargo fmt --check
    npx prettier --check "src/**/*.{ts,svelte,css,html}"

# Lint Rust code
lint:
    cd src-tauri && cargo clippy -- -D warnings

# Type check everything
typecheck:
    npm run check
    cd src-tauri && cargo check

# Run all checks (format, lint, typecheck) - use before submitting work
check-all: fmt-check lint typecheck

# ============================================================================
# Release (Signed & Notarized)
# ============================================================================

# Build signed and notarized app (requires APPLE_* env vars)
# Set these env vars:
#   APPLE_TEAM_ID       - Your 10-char team ID
#   APPLE_IDENTITY      - "Developer ID Application: Your Name (TEAMID)"
#   APPLE_ID            - Your Apple ID email
#   APPLE_ID_PASSWORD   - App-specific password
build-signed:
    npm run tauri build -- --bundles app
    @echo "Signing..."
    codesign --force --deep --timestamp --options=runtime \
        --sign "$APPLE_IDENTITY" \
        src-tauri/target/release/bundle/macos/staged.app
    @echo "Notarizing..."
    ditto -c -k --keepParent src-tauri/target/release/bundle/macos/staged.app staged-notarize.zip
    xcrun notarytool submit staged-notarize.zip \
        --apple-id "$APPLE_ID" \
        --password "$APPLE_ID_PASSWORD" \
        --team-id "$APPLE_TEAM_ID" \
        --wait
    xcrun stapler staple src-tauri/target/release/bundle/macos/staged.app
    rm staged-notarize.zip
    @echo "Verifying..."
    codesign -vvv --deep --strict src-tauri/target/release/bundle/macos/staged.app
    @echo "✅ Signed and notarized: src-tauri/target/release/bundle/macos/staged.app"

# ============================================================================
# Setup & Maintenance
# ============================================================================

# Install dependencies
install:
    npm install
    cd src-tauri && cargo fetch

# Clean build artifacts
clean:
    rm -rf dist
    rm -rf src-tauri/target
    rm -rf node_modules
