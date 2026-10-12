# WorkToJiraEffort

Automatically track your work time via Screenpipe and log effort to Jira & Salesforce.

## Overview

WorkToJiraEffort is a cross-platform (Linux, Windows, macOS) application written in Rust that:
- **Automatically manages Screenpipe** - locates the [Screenpipe](https://github.com/mediar-ai/screenpipe) binary and runs it as a managed subprocess, installing it on first use if it is missing
- Monitors your work activities using Screenpipe
- Automatically detects Jira issue keys from your active applications
- Logs work time to Jira issues
- Optionally logs time entries to Salesforce
- Runs continuously in the background with configurable polling intervals

## Features

- 🔄 **Automatic Time Tracking**: Monitors your active windows and applications via Screenpipe
- 🎯 **Smart Jira Detection**: Automatically finds Jira issue keys (e.g., PROJ-123) in window titles
- 📊 **Dual Platform Support**: Logs time to both Jira and Salesforce
- ⚙️ **Configurable**: Customizable polling intervals and minimum activity duration
- 🔒 **Secure**: API credentials stored in local configuration file
- 🖥️ **Cross-Platform**: Works on Linux, Windows, and macOS
- 🚀 **Managed Screenpipe**: the app installs Screenpipe on first run if it is missing, which requires internet access
- 🎛️ **Menubar/System Tray App**: Optional GUI for easy control and issue override management
- 🔌 **Daemon Mode**: Run as a background service with HTTP API for external control

## Prerequisites

1. **Jira Account** (optional but recommended):
   - Jira instance URL
   - Email address
   - API token (generate at: https://id.atlassian.com/manage-profile/security/api-tokens)

2. **Salesforce Account** (optional):
   - Instance URL
   - Username and password
   - Security token
   - Connected app credentials (client ID and secret)

**Note**: The app manages a Screenpipe instance for you and does not require you to install it in advance. If no `screenpipe` binary is found, it downloads and runs the upstream installer on first use (Unix: `curl ... | sh`, Windows: `iwr ... | iex`), so the first run needs internet access. Screenpipe is not bundled in the build.

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/UltraInstinct0x/WorkToJiraEffort.git
cd WorkToJiraEffort

# Build the project
cargo build --release

# The binary will be at target/release/work-to-jira-effort
```

### Using Cargo

```bash
cargo install --path .
```

## Configuration

### 1. Initialize Configuration

```bash
work-to-jira-effort init
```

This creates a configuration file at:
- **Linux/macOS**: `~/.config/worktojiraeffort/config.toml`
- **Windows**: `%APPDATA%\worktojiraeffort\config.toml`

### 2. Edit Configuration

Open the config file and update with your credentials:

```toml
[screenpipe]
url = "http://localhost:3030"  # Your Screenpipe instance URL

[jira]
url = "https://your-domain.atlassian.net"
email = "your-email@example.com"
api_token = "your-api-token"
enabled = true

[salesforce]
instance_url = "https://your-instance.salesforce.com"
username = "your-username"
password = "your-password"
security_token = "your-security-token"
client_id = "your-client-id"
client_secret = "your-client-secret"
enabled = false  # Set to true to enable Salesforce integration

[tracking]
poll_interval_secs = 300      # Check for new activities every 5 minutes
min_activity_duration_secs = 60  # Only log activities longer than 1 minute
```

## Usage

### Check Configuration and Connectivity

```bash
work-to-jira-effort check
```

This verifies:
- Configuration file is valid
- Screenpipe is accessible
- Jira API credentials are correct
- Salesforce API credentials are correct (if enabled)

The Screenpipe check starts a temporary Screenpipe process and then stops it, rather than only probing an already-running instance.

### Start Tracking

```bash
work-to-jira-effort start
```

The application will:
1. Start the Screenpipe server automatically (installing it first if needed)
2. Connect to Screenpipe
3. Poll for recent activities at the configured interval
4. Consolidate activities by application/window
5. Extract Jira issue keys from window titles
6. Log time to matching Jira issues
7. Log time to Salesforce (if enabled)
8. Stop Screenpipe gracefully when you exit

Press `Ctrl+C` to stop tracking.

### Enable Logging

For detailed logging output:

```bash
RUST_LOG=info work-to-jira-effort start
```

Available log levels: `error`, `warn`, `info`, `debug`, `trace`

### Run as Daemon (Background Service)

For continuous background operation with external control:

```bash
work-to-jira-effort daemon --port 8787
```

The daemon provides:
- **Background tracking**: Runs continuously without user interaction
- **HTTP API**: Control API on `http://127.0.0.1:8787`
  - `GET /status` - Get current status and issue override
  - `POST /issue` - Set or clear Jira issue override
- **External control**: Can be controlled by menubar apps or custom scripts

Example API usage:
```bash
# Get current status
curl http://127.0.0.1:8787/status

# Set issue override
curl -X POST http://127.0.0.1:8787/issue \
  -H 'Content-Type: application/json' \
  -d '{"issue_key": "PROJ-123"}'

# Clear override
curl -X POST http://127.0.0.1:8787/issue \
  -H 'Content-Type: application/json' \
  -d '{"issue_key": null}'
```

### Tauri Desktop Application (Recommended)

For the best user experience, use the modern Tauri desktop application with a beautiful TimeScribe-inspired UI:

```bash
# Build the Tauri app
cargo build --release --bin work-to-jira-effort-ui --features tauri-ui

# Run the app
./target/release/work-to-jira-effort-ui
```

#### UI Features

The Tauri app provides a dual-window architecture inspired by TimeScribe's menubar design:

**🎯 Menubar Popup (Left-Click Tray Icon)**
- **Compact Interface**: 350x450px quick-access popup positioned near tray icon
- **Status at a Glance**: Current tracking status, active issue, and today's time
- **Quick Actions**: Set/clear issue override with single input field
- **Dashboard Access**: "Open Dashboard" button for detailed view
- **Auto-Hide**: Closes when clicking outside or switching focus

**📊 Full Dashboard (Right-Click → Open Dashboard)**
- **Status Overview**: Real-time tracking status and current issue
- **Recent Issues**: Quick-access list of recently tracked issues with time totals
- **Time Summary**: Daily breakdown by issue with visual progress bars
- **Settings & Preferences**: Issue override, notification controls, theme toggle
- **Connection Health**: Live daemon status with pulsing indicator

**🎨 Modern Design**
- TimeScribe-inspired warm terracotta & sage green palette
- Fraunces display font for elegant typography
- Smooth animations and transitions
- Dark/light mode support with system preference detection

**🔄 Real-time Updates**
- Auto-refresh every 30 seconds
- Live connection status monitoring
- Last sync timestamp display

**Platform requirements:**
- **macOS**: 10.13+, no additional dependencies
- **Windows**: Windows 10+, no additional dependencies
- **Linux**: WebKit2GTK 4.0

For UI design specifications and component documentation, see:
- [UI Design System](docs/UI_DESIGN.md)
- [Component Documentation](docs/UI_COMPONENTS.md)
- [Development Guide](docs/DEVELOPMENT.md)

### System Tray Application (Alternative)

For a minimal system tray experience:

```bash
# Build with tray support (requires GUI libraries)
cargo build --release --features tray

# Run the tray app
./target/release/work-to-jira-effort-tray
```

The tray app provides:
- **Auto-start daemon**: Automatically launches the background daemon
- **Visual status**: See current issue override at a glance
- **Quick actions**: Set/clear issue overrides with one click
- **Common issues**: Shortcuts for frequently used Jira issues

For detailed platform-specific build instructions, see [MENUBAR_BUILD.md](MENUBAR_BUILD.md).

**Platform requirements:**
- **macOS**: No additional dependencies
- **Windows**: No additional dependencies
- **Linux**: Requires GTK3 and related libraries (see [MENUBAR_BUILD.md](MENUBAR_BUILD.md))

## How It Works

### Activity Detection

WorkToJiraEffort queries the Screenpipe API to retrieve recent activities including:
- Window titles
- Application names
- Text content from OCR (if available)
- Timestamps

### Jira Integration

The application automatically detects Jira issue keys using the pattern `[A-Z]+-\d+` (e.g., `PROJ-123`, `DEV-456`).

When a match is found in the window title or application name, it:
1. Consolidates activity duration
2. Creates a worklog entry in Jira
3. Includes context about the tracked application

### Salesforce Integration

If enabled, time entries are created in Salesforce using the `TimeEntry__c` custom object. 

**Note**: You may need to customize the Salesforce object name and fields based on your organization's setup. Edit `src/salesforce.rs` to match your schema.

## Architecture

The application is structured into several modules:

- **config**: Configuration management and persistence
- **screenpipe**: Screenpipe API client for activity retrieval
- **screenpipe_manager**: Screenpipe binary discovery, installation and lifecycle management
- **jira**: Jira API client for worklog creation
- **salesforce**: Salesforce API client for time entry creation
- **tracker**: Core tracking logic and activity consolidation with issue override support
- **daemon**: HTTP API server for external control (daemon mode)
- **main**: CLI interface and command handling
- **bin/tray**: System tray/menubar application (optional, requires `tray` feature)

## Security Considerations

- API credentials are stored in plain text in the config file
- Ensure the config file has appropriate permissions (readable only by you)
- Consider using environment variables for credentials in production
- The application does not transmit data to any third parties except Jira and Salesforce

## Troubleshooting

### First Time Setup

On first run, the application will automatically:
- Download and install Screenpipe if not already present
- Set up the necessary data directories
- Start Screenpipe in the background

This may take a few moments. Subsequent runs will be much faster.

### Screenpipe Issues

The application manages Screenpipe automatically. If you encounter issues:
1. The app will attempt to install Screenpipe automatically
2. Check logs for any installation errors
3. Ensure you have internet connectivity for first-time installation
4. If automatic installation fails, you can manually install from: https://github.com/mediar-ai/screenpipe

### Jira Authentication Failed

- Verify your API token is correct
- Ensure your email matches your Jira account
- Check that your Jira instance URL is correct (include https://)

### No Activities Being Tracked

- Verify Screenpipe is recording activities
- Check the `poll_interval_secs` is reasonable (default: 300 seconds)
- Ensure `min_activity_duration_secs` is not too high

### Jira Issues Not Detected

- Ensure issue keys appear in window titles (e.g., "PROJ-123: Feature Implementation")
- Check the regex pattern in `src/jira.rs` if you use a different format

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Code Formatting

```bash
cargo fmt
```

### Linting

```bash
cargo clippy
```

## Implemented in repository

- A Rust CLI with `init`, `start`, `check` and `daemon` commands.
- Jira issue key detection from window titles using the `[A-Z]+-\d+` pattern, then Jira worklog creation.
- Optional Salesforce time entry creation via the `TimeEntry__c` object.
- A daemon mode with an HTTP control API on `http://127.0.0.1:8787` (`GET /status`, `POST /issue`).
- A Tauri desktop UI building on the `tauri-ui` feature and a tray app on the `tray` feature.
- Screenpipe lifecycle management: binary discovery on `PATH` and common locations, and download plus installation of the upstream installer when it is missing.

## Limitations and unverified behavior

- **Screenpipe is not embedded.** The release artifacts contain only the Rust binary; no Screenpipe component is bundled (`resources/` holds a single macOS `Info.plist`). On first run the app downloads and executes the upstream Screenpipe installer, which requires internet access plus `curl`/`sh` on Unix or PowerShell on Windows. `EMBEDDED_SCREENPIPE.md` lists "Bundled Screenpipe binary in release" and "Offline installation support" as future enhancements, so "embedded" here means automatic lifecycle management, not packaging.
- **`check` runs a temporary Screenpipe.** The command starts a Screenpipe subprocess and stops it, rather than only probing one that is already running.
- **Conflicting guidance.** `install.sh` prints "Ensure Screenpipe is running (http://localhost:3030)" on success, which conflicts with the zero-setup wording elsewhere.
- **Credentials are stored in plain text** in the config file. See Security Considerations.
- **Not verified by running.** The project was not built and no tests were executed as part of documenting this, so runtime behavior is not confirmed here.

## Roadmap (proposals, not commitments)

These are proposals, not commitments or scheduled work:

- Bundle a Screenpipe binary in release artifacts and support offline installation.
- Add integration tests.

## Contributing

Contributions are welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

## License

MIT License - see LICENSE file for details

## Acknowledgments

- [Screenpipe](https://github.com/mediar-ai/screenpipe) - Activity monitoring engine
- Jira REST API
- Salesforce REST API

## Support

For issues and questions:
- GitHub Issues: https://github.com/UltraInstinct0x/WorkToJiraEffort/issues

