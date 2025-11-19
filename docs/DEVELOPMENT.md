# Development Guide

## Quick Start

### Prerequisites
- Rust 1.75+ (for Tauri backend)
- Node.js 18+ (optional, for development tools)
- macOS 10.13+ or Windows 10+

### Setup
```bash
# Clone the repository
git clone https://github.com/yourusername/WorkToJiraEffort.git
cd WorkToJiraEffort

# Install Rust dependencies
cargo build

# Install Tauri CLI
cargo install tauri-cli --version "^2.0"

# Run the app
cargo tauri dev
```

---

## Project Structure

```
WorkToJiraEffort/
├── src/                    # Rust backend code
│   ├── main.rs            # CLI entry point
│   ├── daemon.rs          # Background daemon
│   ├── tracker.rs         # Time tracking logic
│   ├── jira.rs            # Jira API integration
│   ├── salesforce.rs      # Salesforce integration
│   ├── screenpipe.rs      # Screen activity monitoring
│   └── bin/
│       ├── tray.rs        # System tray app
│       └── tauri_app.rs   # Tauri UI app
├── ui/                     # Frontend code
│   ├── index.html         # Main HTML
│   ├── styles.css         # Design system styles
│   ├── app.js             # Application logic
│   ├── fonts/             # Custom fonts
│   └── icons/             # SVG icons
├── docs/                   # Documentation
│   ├── UI_DESIGN.md       # Design system
│   ├── UI_COMPONENTS.md   # Component specs
│   └── DEVELOPMENT.md     # This file
├── icons/                  # App icons
├── config.example.toml     # Config template
├── Cargo.toml             # Rust dependencies
└── tauri.conf.json        # Tauri configuration
```

---

## Development Workflow

### Running the App

#### Development Mode
```bash
# Run with hot reload
cargo tauri dev

# Run daemon separately for testing
cargo run -- daemon --port 8787
```

#### Production Build
```bash
# Build for release
cargo tauri build

# Build specific binary
cargo build --release --bin work-to-jira-effort-ui --features tauri-ui
```

### Frontend Development

The UI is built with vanilla HTML/CSS/JS for simplicity and performance.

#### File Watching
The Tauri dev server automatically watches `ui/` directory for changes.

#### Live Reload
Save any file in `ui/` and the app will reload automatically.

---

## UI Development Guidelines

### Adding a New Component

1. **Document the component** in `docs/UI_COMPONENTS.md`
   - Purpose
   - Structure (HTML)
   - Styling (CSS)
   - Behavior (JS)
   - States (loading, error, empty)

2. **Add HTML structure** to `ui/index.html`
   ```html
   <div class="new-component">
     <!-- Component markup -->
   </div>
   ```

3. **Add styles** to `ui/styles.css`
   ```css
   .new-component {
     /* Use design system tokens */
     background: var(--color-surface);
     padding: var(--space-4);
     border-radius: var(--radius-md);
   }
   ```

4. **Add behavior** to `ui/app.js`
   ```javascript
   function initNewComponent() {
     // Component logic
   }
   ```

5. **Test the component**
   - Light mode
   - Dark mode
   - Empty state
   - Loading state
   - Error state
   - Accessibility

### Styling Best Practices

1. **Use CSS Custom Properties**
   ```css
   /* Good */
   color: var(--color-text);
   padding: var(--space-4);

   /* Bad */
   color: #292524;
   padding: 16px;
   ```

2. **Follow BEM Naming**
   ```css
   .component {}
   .component__element {}
   .component__element--modifier {}
   ```

3. **Mobile-First**
   ```css
   /* Base styles for mobile */
   .component {
     padding: var(--space-2);
   }

   /* Desktop enhancements */
   @media (min-width: 640px) {
     .component {
       padding: var(--space-4);
     }
   }
   ```

4. **Smooth Transitions**
   ```css
   .component {
     transition: all var(--duration-normal) var(--ease-out);
   }
   ```

---

## Backend Development

### Adding a Tauri Command

1. **Define the command** in `src/bin/tauri_app.rs`
   ```rust
   #[tauri::command]
   async fn new_command(param: String) -> Result<Response, String> {
       // Implementation
       Ok(Response { data: "result" })
   }
   ```

2. **Register in handler**
   ```rust
   .invoke_handler(tauri::generate_handler![
       get_status,
       new_command,  // Add here
   ])
   ```

3. **Call from frontend**
   ```javascript
   const result = await invoke('new_command', { param: 'value' });
   ```

### Data Models

Define shared types in `src/bin/tauri_app.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NewModel {
    field: String,
}
```

---

## Testing

### Manual Testing Checklist

#### Functionality
- [ ] App launches successfully
- [ ] Daemon connection works
- [ ] Status updates in real-time
- [ ] Issue override sets correctly
- [ ] Recent issues persist
- [ ] Time summary calculates correctly
- [ ] Notifications work (if enabled)

#### UI/UX
- [ ] Light mode renders correctly
- [ ] Dark mode renders correctly
- [ ] Animations are smooth
- [ ] No layout shift on load
- [ ] Empty states show appropriately
- [ ] Loading states display
- [ ] Error messages are clear

#### Accessibility
- [ ] Keyboard navigation works
- [ ] Focus indicators visible
- [ ] Screen reader announces changes
- [ ] Color contrast meets WCAG AA
- [ ] Text is resizable

#### Performance
- [ ] App starts quickly (< 2s)
- [ ] UI is responsive (60fps)
- [ ] No memory leaks
- [ ] CPU usage is reasonable

### Automated Testing

```bash
# Run Rust tests
cargo test

# Run clippy for linting
cargo clippy -- -D warnings

# Format code
cargo fmt
```

---

## Debugging

### Frontend Debugging

#### Enable DevTools
```javascript
// In tauri.conf.json, temporarily enable:
{
  "build": {
    "devPath": "http://localhost:3000"
  }
}
```

Then open DevTools with `Cmd+Option+I` (macOS) or `F12` (Windows).

#### Console Logging
```javascript
console.log('Debug info:', data);
console.error('Error:', error);
```

### Backend Debugging

#### Enable Logging
```bash
# Set log level
export RUST_LOG=debug
cargo tauri dev
```

#### Print Debugging
```rust
println!("Debug: {:?}", variable);
eprintln!("Error: {:?}", error);
```

#### VSCode Debugging
Add to `.vscode/launch.json`:
```json
{
  "type": "lldb",
  "request": "launch",
  "name": "Debug Tauri App",
  "cargo": {
    "args": ["build", "--bin", "work-to-jira-effort-ui", "--features", "tauri-ui"]
  }
}
```

---

## Git Workflow

### Branch Strategy
- `main` - Stable releases
- `feature/*` - New features
- `bugfix/*` - Bug fixes
- `hotfix/*` - Emergency fixes

### Commit Messages
Follow conventional commits:
```
feat(ui): add time summary visualization
fix(daemon): resolve connection timeout
docs(readme): update installation instructions
refactor(tracker): improve accuracy algorithm
```

### Pull Request Template
```markdown
## Summary
Brief description of changes

## Changes
- Change 1
- Change 2

## Testing
- [ ] Manual testing completed
- [ ] All tests pass
- [ ] Accessibility checked

## Screenshots
Before/after if UI changes
```

---

## Building for Production

### macOS App Bundle
```bash
# Build and create .app
cargo tauri build

# Output: target/release/bundle/macos/WorkToJiraEffort.app
```

### Windows Installer
```bash
# Build and create .exe installer
cargo tauri build

# Output: target/release/bundle/msi/WorkToJiraEffort.msi
```

### Code Signing (macOS)
```bash
# Sign the app
codesign --force --deep --sign "Developer ID Application: Your Name" \
  target/release/bundle/macos/WorkToJiraEffort.app

# Notarize with Apple
xcrun notarytool submit WorkToJiraEffort.app.zip \
  --apple-id "your@email.com" \
  --password "app-specific-password" \
  --team-id "TEAM_ID"
```

---

## Performance Optimization

### Frontend
- Minimize DOM manipulations
- Use CSS transforms for animations
- Debounce API calls
- Lazy load images/icons

### Backend
- Use async/await for I/O
- Cache API responses
- Batch database operations
- Profile with `cargo flamegraph`

---

## Troubleshooting

### App Won't Launch
1. Check daemon is running: `curl http://localhost:8787/status`
2. Check Tauri logs: `~/Library/Logs/WorkToJiraEffort/`
3. Rebuild: `cargo clean && cargo tauri build`

### UI Not Updating
1. Check browser console for errors
2. Verify Tauri commands are registered
3. Check network tab for API calls
4. Clear localStorage: `localStorage.clear()`

### Performance Issues
1. Profile with DevTools Performance tab
2. Check for memory leaks
3. Optimize animations (use `will-change`)
4. Reduce polling frequency

---

## Resources

- [Tauri Documentation](https://tauri.app/)
- [Rust Book](https://doc.rust-lang.org/book/)
- [MDN Web Docs](https://developer.mozilla.org/)
- [Jira REST API](https://developer.atlassian.com/cloud/jira/platform/rest/v3/)

---

## Contributing

See `CONTRIBUTING.md` for guidelines on:
- Code style
- Commit conventions
- PR process
- Code review expectations
