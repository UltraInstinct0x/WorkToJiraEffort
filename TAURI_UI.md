# Tauri UI for WorkToJiraEffort

This is a lightweight desktop UI built with Tauri, providing a TimeScribe-like interface for WorkToJiraEffort.

## Features

- **System Tray Integration**: Lives in your menubar/system tray
- **Modern UI**: Clean, lightweight interface with HTML/CSS/JavaScript
- **Real-time Status**: Shows current tracking status and active issue
- **Quick Actions**: Easily switch between issues or clear overrides
- **Small Footprint**: Lightweight compared to Electron alternatives

## Architecture

The Tauri app consists of:

1. **Rust Backend** (`src/bin/tauri_app.rs`): Handles system tray, API calls to daemon
2. **Frontend** (`ui/`): HTML/CSS/JavaScript interface
3. **Icons** (`icons/`): App and tray icons

## Building

### Prerequisites

On Linux, you need GTK development libraries:
```bash
# Ubuntu/Debian
sudo apt-get install libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

# Fedora
sudo dnf install gtk3-devel webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel patchelf

# Arch
sudo pacman -S gtk3 webkit2gtk libappindicator-gtk3 librsvg patchelf
```

On macOS, no additional dependencies needed.

### Build Commands

```bash
# Build in debug mode
cargo build --bin work-to-jira-effort-ui --features tauri-ui

# Build in release mode (optimized)
cargo build --bin work-to-jira-effort-ui --features tauri-ui --release

# Run directly
cargo run --bin work-to-jira-effort-ui --features tauri-ui
```

## Running

1. **Start the daemon** (if not already running):
   ```bash
   cargo run -- daemon --port 8787
   ```

2. **Launch the Tauri UI**:
   ```bash
   # Debug build
   ./target/debug/work-to-jira-effort-ui

   # Release build
   ./target/release/work-to-jira-effort-ui
   ```

3. **Look for the tray icon** in your menubar/system tray (blue square icon)

4. **Click the icon** to open the UI window

## UI Features

### Main Dashboard

- **Current Status Card**:
  - Shows active issue (or "Auto-detect" if no override)
  - Tracking status indicator
  - Today's total tracked time

- **Issue Selection**:
  - Text input to set custom issue
  - Quick action buttons for common issues (PROJ-123, PROJ-456, PROJ-789)
  - Clear override button

- **Auto-refresh**: Status refreshes every 30 seconds automatically

### System Tray Menu

Right-click the tray icon for:
- **Show**: Open the UI window
- **Quit**: Exit the application

## Customization

### Change Quick Action Issues

Edit `ui/index.html` and modify the quick action buttons:

```html
<button class="btn btn-secondary btn-sm" data-issue="YOUR-ISSUE">YOUR-ISSUE</button>
```

### Update UI Styling

Edit `ui/styles.css` to change colors, fonts, or layout.

### Modify Icon

Replace files in `icons/` directory:
- `icon.png`: Main icon (used for Linux/Windows)
- `icon.icns`: macOS icon
- `icon.ico`: Windows icon
- Various sizes: `32x32.png`, `128x128.png`, etc.

Or run the icon generation script:
```bash
python3 create_icons.py
```

## Architecture Details

### Communication Flow

```
UI (JavaScript) <--Tauri IPC--> Rust Backend <--HTTP--> Daemon (Port 8787)
```

### Tauri Commands

The following commands are available from JavaScript:

- `get_status()`: Get current daemon status
- `set_issue_override(issue_key)`: Set issue override
- `get_activity_summary()`: Get tracking summary

### Adding New Features

1. **Add Rust command** in `src/bin/tauri_app.rs`:
   ```rust
   #[tauri::command]
   async fn your_command() -> Result<YourType, String> {
       // Implementation
   }
   ```

2. **Register command** in the `invoke_handler`:
   ```rust
   .invoke_handler(tauri::generate_handler![
       get_status,
       your_command  // Add here
   ])
   ```

3. **Call from JavaScript** in `ui/app.js`:
   ```javascript
   const result = await invoke('your_command');
   ```

## Comparison with Other Approaches

| Approach | Size | Complexity | Native Feel | Development Speed |
|----------|------|-----------|-------------|-------------------|
| **Tauri** | ~10MB | Medium | Good | Fast |
| egui | ~5MB | Medium | Fair | Medium |
| Electron | ~100MB+ | Low | Good | Fast |
| Native (Swift/GTK) | ~2MB | High | Excellent | Slow |

Tauri offers the best balance of size, development speed, and native feel.

## Troubleshooting

### Linux: Tray icon not showing

Some desktop environments don't support tray icons well. Try:
- Install `libappindicator3-1` or equivalent
- Use KDE Plasma, GNOME with extensions, or XFCE

### macOS: App not starting

Make sure you're using macOS 10.13 or later.

### Build errors

1. Clean build:
   ```bash
   cargo clean
   cargo build --bin work-to-jira-effort-ui --features tauri-ui
   ```

2. Update dependencies:
   ```bash
   cargo update
   ```

## Future Enhancements

Potential additions:
- [ ] Charts/graphs for time tracking visualization
- [ ] Activity timeline view
- [ ] Settings panel
- [ ] Notifications for tracking events
- [ ] Dark mode toggle
- [ ] Custom issue templates
- [ ] Keyboard shortcuts
- [ ] Multi-monitor support

## License

Same as WorkToJiraEffort main project (MIT).
