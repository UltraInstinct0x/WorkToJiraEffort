# Troubleshooting Guide

This guide helps you diagnose and fix common issues with WorkToJiraEffort.

## General Debugging

Enable debug logging to see detailed information:

```bash
RUST_LOG=debug work-to-jira-effort start
```

## Common Issues

### 1. Configuration Not Found

**Error**: `Failed to read config file`

**Solution**:
```bash
# Initialize configuration
work-to-jira-effort init

# Edit the configuration file
# Linux/macOS: ~/.config/worktojiraeffort/config.toml
# Windows: %APPDATA%\worktojiraeffort\config.toml
```

### 2. Screenpipe Connection Failed

**Error**: `Failed to fetch activities from Screenpipe`

**Diagnosis**:
```bash
# Check if Screenpipe is running
curl http://localhost:3030/health

# Or using the built-in check
work-to-jira-effort check
```

**Solutions**:
- Ensure Screenpipe is installed and running
- Verify the URL in your config.toml matches your Screenpipe instance
- Check if port 3030 is accessible
- Review Screenpipe logs for errors

### 3. Jira Authentication Failed

**Error**: `Jira API error (401)` or `Jira API error (403)`

**Solutions**:
- Verify your email address is correct
- Regenerate your Jira API token at: https://id.atlassian.com/manage-profile/security/api-tokens
- Ensure you're using the correct Jira instance URL
- Check that your Jira account has permission to log work

**Test manually**:
```bash
curl -u your-email@example.com:your-api-token \
  https://your-domain.atlassian.net/rest/api/3/myself
```

### 4. Salesforce Authentication Failed

**Error**: `Salesforce authentication error`

**Solutions**:
- Verify your username and password are correct
- Ensure you've included the security token (appended to password internally)
- Check that your Connected App is properly configured
- Verify client_id and client_secret are correct

**Common Salesforce issues**:
- Security token expired - reset at: Setup → Personal Setup → Reset My Security Token
- IP restrictions - check your organization's IP whitelist
- API access disabled - verify API is enabled for your user

### 5. No Activities Being Tracked

**Possible Causes**:
- Screenpipe isn't recording activities
- Activities are shorter than `min_activity_duration_secs`
- Poll interval is too long

**Solutions**:
```bash
# Check Screenpipe is recording
curl http://localhost:3030/search

# Adjust settings in config.toml
[tracking]
poll_interval_secs = 60        # Poll every minute
min_activity_duration_secs = 30  # Log activities > 30 seconds
```

### 6. Jira Issues Not Being Detected

**Error**: Activities logged but not appearing in Jira

**Possible Causes**:
- Issue keys not in window titles
- Regex pattern not matching your issue format

**Solutions**:
- Ensure issue keys appear in window titles (e.g., "PROJ-123: My Task")
- Check the regex pattern in src/jira.rs: `([A-Z]+-\d+)`
- Verify the issue exists and you have permission to log work

**Test issue detection**:
```bash
# Your window title should contain something like:
# "PROJ-123: Feature Implementation - Chrome"
# "Working on DEV-456 - VSCode"
```

### 7. Build Errors

**Error**: `cargo build` fails

**Solutions**:
```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build

# Check Rust version (requires 1.70+)
rustc --version
```

### 8. Permission Denied Errors

**Error**: Cannot write to config directory

**Solutions**:
```bash
# Linux/macOS - Check permissions
ls -la ~/.config/worktojiraeffort/

# Create directory manually if needed
mkdir -p ~/.config/worktojiraeffort
chmod 700 ~/.config/worktojiraeffort
```

### 9. SSL/TLS Certificate Errors

**Error**: `SSL certificate problem`

**Solutions**:
- Update system certificates
- Check if you're behind a corporate proxy
- Verify Jira/Salesforce URLs use HTTPS

### 10. Application Crashes

**Solutions**:
1. Check logs with debug mode:
   ```bash
   RUST_LOG=debug work-to-jira-effort start 2>&1 | tee debug.log
   ```

2. Report the issue with:
   - Debug log output
   - Operating system and version
   - Rust version (`rustc --version`)
   - Steps to reproduce

## Getting Help

If you're still experiencing issues:

1. **Check existing issues**: https://github.com/UltraInstinct0x/WorkToJiraEffort/issues
2. **Create a new issue** with:
   - Clear description of the problem
   - Steps to reproduce
   - Log output (with sensitive data removed)
   - Your environment (OS, Rust version)

## Testing Your Setup

Complete verification checklist:

```bash
# 1. Initialize configuration
work-to-jira-effort init

# 2. Edit config with your credentials
# (Edit ~/.config/worktojiraeffort/config.toml)

# 3. Check connectivity
work-to-jira-effort check

# Expected output:
# ✓ Screenpipe: ✓
# ✓ Jira: ✓
# ✓ Salesforce: ✓ (if enabled)

# 4. Start tracking
RUST_LOG=info work-to-jira-effort start
```

## Advanced Debugging

### Inspect HTTP Requests

Use a proxy like mitmproxy to inspect API calls:

```bash
# Install mitmproxy
pip install mitmproxy

# Run mitmproxy
mitmproxy -p 8080

# Configure app to use proxy
export http_proxy=http://localhost:8080
export https_proxy=http://localhost:8080
work-to-jira-effort start
```

### Check Network Connectivity

```bash
# Test Jira
curl -v https://your-domain.atlassian.net/rest/api/3/myself \
  -u your-email@example.com:your-api-token

# Test Salesforce
curl -v https://your-instance.salesforce.com/services/oauth2/token \
  -X POST \
  -d "grant_type=password&client_id=...&client_secret=...&username=...&password=..."
```

## Performance Issues

If the application is slow:

1. **Reduce poll frequency**:
   ```toml
   [tracking]
   poll_interval_secs = 600  # 10 minutes instead of 5
   ```

2. **Increase minimum duration**:
   ```toml
   [tracking]
   min_activity_duration_secs = 120  # 2 minutes instead of 1
   ```

3. **Disable unused integrations**:
   ```toml
   [salesforce]
   enabled = false
   ```
