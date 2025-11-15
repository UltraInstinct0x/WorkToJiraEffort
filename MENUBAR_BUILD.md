# Building the Menubar/System Tray Application

The WorkToJiraEffort menubar application provides a cross-platform system tray interface for controlling the tracking daemon and managing Jira issue overrides.

## Architecture

The application consists of two components:

1. **Daemon** (`work-to-jira-effort daemon`) - Background service that:
   - Runs the work tracker continuously
   - Manages embedded Screenpipe server
   - Exposes HTTP API on `http://127.0.0.1:8787`

2. **Tray App** (`work-to-jira-effort-tray`) - System tray GUI that:
   - Auto-starts the daemon
   - Shows current status and issue override
   - Allows setting/clearing issue overrides
   - Provides quick access to common tasks

## Platform-Specific Build Instructions

### macOS

macOS has native system tray support with no additional dependencies required.

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build with tray feature
cargo build --release --features tray

# Run the tray app
./target/release/work-to-jira-effort-tray
```

The tray icon will appear in the macOS menu bar (top right corner).

#### Building an App Bundle (Optional)

To create a proper macOS application:

```bash
# Build release binary
cargo build --release --features tray

# Create app bundle structure
mkdir -p WorkToJiraEffort.app/Contents/MacOS
mkdir -p WorkToJiraEffort.app/Contents/Resources

# Copy binaries
cp target/release/work-to-jira-effort-tray WorkToJiraEffort.app/Contents/MacOS/
cp target/release/work-to-jira-effort WorkToJiraEffort.app/Contents/MacOS/

# Create Info.plist
cat > WorkToJiraEffort.app/Contents/Info.plist << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>work-to-jira-effort-tray</string>
    <key>CFBundleIdentifier</key>
    <string>com.worktojiraeffort.tray</string>
    <key>CFBundleName</key>
    <string>WorkToJiraEffort</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSUIElement</key>
    <true/>
</dict>
</plist>
EOF

# Run the app
open WorkToJiraEffort.app
```

### Windows

Windows supports system tray natively through the Windows API.

```powershell
# Install Rust if not already installed
# Download from https://rustup.rs/

# Build with tray feature
cargo build --release --features tray

# Run the tray app
.\target\release\work-to-jira-effort-tray.exe
```

The tray icon will appear in the Windows system tray (bottom right corner).

#### Creating a Windows Installer (Optional)

Use WiX Toolset or Inno Setup to create an installer:

```powershell
# Using cargo-wix
cargo install cargo-wix
cargo wix --features tray
```

### Linux

Linux requires GTK and related libraries for system tray support.

#### Ubuntu/Debian

```bash
# Install dependencies
sudo apt-get update
sudo apt-get install -y \
    libgtk-3-dev \
    libpango1.0-dev \
    libgdk-pixbuf2.0-dev \
    libatk1.0-dev \
    libcairo2-dev \
    libglib2.0-dev

# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build with tray feature
cargo build --release --features tray

# Run the tray app
./target/release/work-to-jira-effort-tray
```

#### Fedora/RHEL/CentOS

```bash
# Install dependencies
sudo dnf install -y \
    gtk3-devel \
    pango-devel \
    gdk-pixbuf2-devel \
    atk-devel \
    cairo-devel \
    glib2-devel

# Build with tray feature
cargo build --release --features tray

# Run the tray app
./target/release/work-to-jira-effort-tray
```

#### Arch Linux

```bash
# Install dependencies
sudo pacman -S gtk3 pango gdk-pixbuf2 atk cairo glib2

# Build with tray feature
cargo build --release --features tray

# Run the tray app
./target/release/work-to-jira-effort-tray
```

#### Creating a Desktop Entry (Optional)

```bash
# Create desktop entry
cat > ~/.local/share/applications/work-to-jira-effort.desktop << EOF
[Desktop Entry]
Name=WorkToJiraEffort
Comment=Track work time and log to Jira
Exec=/path/to/work-to-jira-effort-tray
Icon=/path/to/icon.png
Type=Application
Categories=Utility;
StartupNotify=false
X-GNOME-Autostart-enabled=true
EOF

# Make it executable
chmod +x ~/.local/share/applications/work-to-jira-effort.desktop
```

## Usage

Once the tray app is running, you'll see a small icon in your system tray/menubar. Click it to access:

- **Current Status**: Shows the currently set Jira issue override (or "No override set")
- **Set Issue Shortcuts**: Quick buttons for common issues (PROJ-123, PROJ-456, PROJ-789)
- **Clear Override**: Remove the current override and return to automatic detection
- **Quit**: Stop the tray app and daemon

## API Endpoints

The daemon exposes a REST API that the tray app (or other clients) can use:

### GET /status

Returns current daemon status:

```json
{
  "version": "0.1.0",
  "issue_override": "PROJ-123"  // or null if not set
}
```

### POST /issue

Set or clear issue override:

```bash
# Set override
curl -X POST http://127.0.0.1:8787/issue \
  -H 'Content-Type: application/json' \
  -d '{"issue_key": "PROJ-123"}'

# Clear override
curl -X POST http://127.0.0.1:8787/issue \
  -H 'Content-Type: application/json' \
  -d '{"issue_key": null}'
```

## Customization

### Changing Issue Shortcuts

Edit `src/bin/tray.rs` and modify the menu items in the `create_menu` function:

```rust
// Change these to your commonly used issues
let proj_123 = MenuItem::new("Set: YOUR-ISSUE-1", true, None);
let proj_456 = MenuItem::new("Set: YOUR-ISSUE-2", true, None);
let proj_789 = MenuItem::new("Set: YOUR-ISSUE-3", true, None);
```

### Custom Icon

Replace the `create_icon_image` function in `src/bin/tray.rs` with your own icon:

```rust
fn create_icon_image() -> tray_icon::Icon {
    let icon_bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(icon_bytes).unwrap();
    // Convert to RGBA and create icon...
}
```

### Daemon Port

By default, the daemon runs on port 8787. To change it:

1. Update `DAEMON_PORT` constant in `src/bin/tray.rs`
2. Or run daemon separately: `work-to-jira-effort daemon --port 9999`

## Troubleshooting

### Tray icon not appearing

- **macOS**: Ensure app has accessibility permissions
- **Windows**: Check Windows notification settings
- **Linux**: Ensure your desktop environment supports system tray (GNOME Shell may need an extension)

### Daemon won't start

Check logs:
```bash
RUST_LOG=debug ./target/release/work-to-jira-effort-tray
```

### Connection refused errors

Ensure daemon is running:
```bash
curl http://127.0.0.1:8787/status
```

## Development

Run in debug mode with logging:

```bash
RUST_LOG=info cargo run --features tray --bin work-to-jira-effort-tray
```

## Next Steps

Future enhancements planned:
- Dynamic Jira issue browser/picker
- Board and sprint selectors
- Time tracking summaries
- Notifications for sync events
- System startup auto-launch
- Code signing and notarization (macOS)
- Windows installer with auto-update
