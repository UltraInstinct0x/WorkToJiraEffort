#!/bin/bash
# Script to create a properly configured macOS app bundle for the tray app
# Uses cargo-bundle for professional app bundling

set -e

echo "=========================================="
echo "Building WorkToJiraEffort macOS App Bundle"
echo "=========================================="
echo ""

# Check if cargo-bundle is installed
if ! command -v cargo-bundle &> /dev/null; then
    echo "cargo-bundle not found. Installing..."
    cargo install cargo-bundle
fi

echo "Building release binaries (Tauri App)..."
cargo build --release --features tauri-ui --bin work-to-jira-effort-ui

echo ""
echo "Creating app bundle with cargo-bundle..."
# We use the tauri-ui binary as the main entry point now
cargo bundle --release --features tauri-ui --bin work-to-jira-effort-ui

# Get the app bundle path
# Cargo bundle with the updated config will create WorkToJiraEffort.app directly
APP_BUNDLE="target/release/bundle/osx/WorkToJiraEffort.app"

if [ ! -d "$APP_BUNDLE" ]; then
    echo "Error: App bundle not created at $APP_BUNDLE"
    exit 1
fi

echo ""
echo "Copying daemon binary to app bundle..."
# We need the daemon binary inside the bundle so the UI can spawn it
cp target/release/work-to-jira-effort "$APP_BUNDLE/Contents/MacOS/"
cp config.example.toml "$APP_BUNDLE/Contents/Resources/" 2>/dev/null || echo "Warning: config.example.toml not found"

echo "Updating Info.plist with menubar app settings..."
# Add LSUIElement to make it a menubar-only app (no Dock icon)
/usr/libexec/PlistBuddy -c "Add :LSUIElement bool true" "$APP_BUNDLE/Contents/Info.plist" 2>/dev/null || \
    /usr/libexec/PlistBuddy -c "Set :LSUIElement true" "$APP_BUNDLE/Contents/Info.plist"

# Update bundle identifier
/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier com.worktojiraeffort.tray" "$APP_BUNDLE/Contents/Info.plist" 2>/dev/null

# Update display name
/usr/libexec/PlistBuddy -c "Set :CFBundleDisplayName WorkToJiraEffort" "$APP_BUNDLE/Contents/Info.plist" 2>/dev/null
/usr/libexec/PlistBuddy -c "Set :CFBundleName WorkToJiraEffort" "$APP_BUNDLE/Contents/Info.plist" 2>/dev/null

echo ""
echo "=========================================="
echo "✅ App bundle created successfully!"
echo "=========================================="
echo ""
echo "Location: $APP_BUNDLE"
echo ""
echo "To run the app:"
echo "  open $APP_BUNDLE"
echo ""
echo "The app will:"
echo "  • Show a blue icon in your menubar"
echo "  • Auto-start the daemon on port 8787"
echo "  • No Dock icon (menubar only)"
echo ""
echo "To stop the app:"
echo "  • Click menubar icon → Quit"
echo ""
