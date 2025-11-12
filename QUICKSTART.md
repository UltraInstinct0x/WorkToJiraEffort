# Quick Start Guide

Get up and running with WorkToJiraEffort in 5 minutes!

## Prerequisites

1. **Jira API Token** (if using Jira integration)
   - Go to: https://id.atlassian.com/manage-profile/security/api-tokens
   - Click "Create API token"
   - Copy the token (you'll need it later)

**Note**: Screenpipe is now embedded! The application will automatically install and manage Screenpipe for you - no separate installation needed!

## Installation

### Option 1: Quick Install (Linux/macOS)

```bash
# Clone the repository
git clone https://github.com/UltraInstinct0x/WorkToJiraEffort.git
cd WorkToJiraEffort

# Run the installer
./install.sh
```

### Option 2: Manual Build

```bash
# Clone the repository
git clone https://github.com/UltraInstinct0x/WorkToJiraEffort.git
cd WorkToJiraEffort

# Build the project
cargo build --release

# The binary is at: target/release/work-to-jira-effort
```

### Option 3: Using Cargo

```bash
# Install from source
cargo install --path .
```

## Configuration

### Step 1: Initialize Config

```bash
work-to-jira-effort init
```

This creates a config file at:
- **Linux/macOS**: `~/.config/worktojiraeffort/config.toml`
- **Windows**: `%APPDATA%\worktojiraeffort\config.toml`

### Step 2: Edit Configuration

Open the config file in your favorite editor:

```bash
# Linux/macOS
nano ~/.config/worktojiraeffort/config.toml

# Windows
notepad %APPDATA%\worktojiraeffort\config.toml
```

### Step 3: Update Credentials

**Minimum required (Jira only):**

```toml
[jira]
url = "https://your-company.atlassian.net"  # Your Jira URL
email = "you@company.com"                    # Your Jira email
api_token = "your-api-token-here"            # Token from step 2 of prerequisites
enabled = true
```

**Optional (Salesforce):**

```toml
[salesforce]
instance_url = "https://your-instance.salesforce.com"
username = "your-username"
password = "your-password"
security_token = "your-security-token"
client_id = "your-client-id"
client_secret = "your-client-secret"
enabled = true  # Set to true to enable
```

## Verify Setup

Check that everything is configured correctly:

```bash
work-to-jira-effort check
```

You should see:
```
✓ Screenpipe: ✓
✓ Jira: ✓
✓ Salesforce: ✓  (if enabled)
```

## Start Tracking

```bash
work-to-jira-effort start
```

The app will now:
1. Automatically start the embedded Screenpipe server
2. Monitor your activities via Screenpipe
3. Look for Jira issue keys in window titles (e.g., PROJ-123)
4. Automatically log time to matching Jira issues
5. Log to Salesforce if enabled
6. Gracefully stop Screenpipe when you exit

## How to Use

### Ensure Issue Keys in Window Titles

For automatic tracking to work, make sure Jira issue keys appear in your window titles:

✅ Good examples:
- "PROJ-123: Feature Implementation - Chrome"
- "Working on DEV-456 - VSCode"
- "[INFRA-789] Server Setup - Terminal"

❌ Won't work:
- "Chrome - Google"
- "VSCode - main.rs"

### Stop Tracking

Press `Ctrl+C` in the terminal running the app.

### Run in Background (Linux/macOS)

```bash
# Start in background
nohup work-to-jira-effort start > /tmp/work-tracker.log 2>&1 &

# Check if running
pgrep -f work-to-jira-effort

# Stop
pkill -f work-to-jira-effort
```

### Run as Service (Linux with systemd)

```bash
# Copy the service file
sudo cp work-to-jira-effort.service /etc/systemd/system/

# Edit the service file to update paths and user
sudo nano /etc/systemd/system/work-to-jira-effort.service

# Enable and start
sudo systemctl enable work-to-jira-effort
sudo systemctl start work-to-jira-effort

# Check status
sudo systemctl status work-to-jira-effort

# View logs
sudo journalctl -u work-to-jira-effort -f
```

## Troubleshooting

### First Run Takes Longer

On the first run, the application will:
- Automatically download and install Screenpipe (if not already present)
- Set up necessary directories
- Start Screenpipe in the background

This is normal and subsequent runs will be much faster.

### Screenpipe Installation Issues

The application will attempt to install Screenpipe automatically. If this fails:
- Ensure you have internet connectivity
- Check that you have write permissions in your home directory
- Manually install Screenpipe from: https://github.com/mediar-ai/screenpipe

### Jira Authentication Failed

1. Verify your email and API token
2. Test manually:
   ```bash
   curl -u you@company.com:your-api-token \
     https://your-company.atlassian.net/rest/api/3/myself
   ```

### No Activities Tracked

1. Check Screenpipe is recording:
   ```bash
   curl http://localhost:3030/search
   ```

2. Lower the minimum duration in config:
   ```toml
   [tracking]
   min_activity_duration_secs = 30
   ```

### Enable Debug Logging

```bash
RUST_LOG=debug work-to-jira-effort start
```

For more help, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md)

## Next Steps

- **Customize** polling interval and filters in config.toml
- **Review** tracked time in Jira
- **Set up** as a system service for automatic startup
- **Check** the full README for advanced features

## Support

- 📚 [Full Documentation](README.md)
- 🐛 [Report Issues](https://github.com/UltraInstinct0x/WorkToJiraEffort/issues)
- 🤝 [Contributing](CONTRIBUTING.md)
