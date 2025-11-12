#!/bin/bash
# Installation script for work-to-jira-effort

set -e

echo "WorkToJiraEffort Installation Script"
echo "===================================="
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust is not installed."
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

echo "✓ Rust is installed"

# Build the project
echo ""
echo "Building work-to-jira-effort..."
cargo build --release

# Check if build was successful
if [ ! -f "target/release/work-to-jira-effort" ]; then
    echo "Error: Build failed"
    exit 1
fi

echo "✓ Build successful"

# Determine installation directory
INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "${INSTALL_DIR}"

# Copy binary
echo ""
echo "Installing to ${INSTALL_DIR}..."
cp target/release/work-to-jira-effort "${INSTALL_DIR}/"
chmod +x "${INSTALL_DIR}/work-to-jira-effort"

echo "✓ Binary installed"

# Check if install directory is in PATH
if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
    echo ""
    echo "Warning: ${INSTALL_DIR} is not in your PATH"
    echo "Add the following line to your ~/.bashrc or ~/.zshrc:"
    echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
fi

# Initialize configuration
echo ""
echo "Initializing configuration..."
"${INSTALL_DIR}/work-to-jira-effort" init

echo ""
echo "✓ Installation complete!"
echo ""
echo "Next steps:"
echo "1. Edit the configuration file with your credentials:"
echo "   ${HOME}/.config/worktojiraeffort/config.toml"
echo ""
echo "2. Ensure Screenpipe is running (http://localhost:3030)"
echo ""
echo "3. Test the configuration:"
echo "   work-to-jira-effort check"
echo ""
echo "4. Start tracking:"
echo "   work-to-jira-effort start"
echo ""
echo "For more information, see the README.md file"
