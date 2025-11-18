# Complete Code Location Reference Map

## Key Files and Their Functions

### 1. screenpipe.rs
**Path:** `/home/user/WorkToJiraEffort/src/screenpipe.rs`
**Size:** 122 lines

| Function | Lines | Purpose |
|----------|-------|---------|
| `Activity` struct | 7-14 | Core activity data model |
| `ScreenpipeResponse` struct | 16-19 | API response wrapper |
| `ScreenpipeSearchEntry` struct | 21-26 | Individual search entry |
| `ScreenpipeContent` struct | 28-36 | Raw content from Screenpipe |
| `ScreenpipeClient::new()` | 44-49 | Constructor |
| **`get_recent_activities()`** | 51-111 | **FETCHES DATA FROM SCREENPIPE** |
| `health_check()` | 113-120 | Connectivity verification |

**Key Data Transformation (lines 88-108):**
```rust
// Incoming: ScreenpipeResponse with raw JSON
// Processing: Filter, map, parse timestamps
// Outgoing: Vec<Activity> with normalized data
```

---

### 2. screenpipe_manager.rs
**Path:** `/home/user/WorkToJiraEffort/src/screenpipe_manager.rs`
**Size:** 225 lines

| Function | Lines | Purpose |
|----------|-------|---------|
| `ScreenpipeManager` struct | 7-11 | Lifecycle manager |
| **`start()`** | 22-89 | **STARTS SCREENPIPE SUBPROCESS** |
| `find_screenpipe_binary()` | 92-119 | Locates binary |
| `install_screenpipe()` | 122-169 | Auto-installation |
| **`stop()`** | 172-215 | **STOPS SCREENPIPE SUBPROCESS** |
| Drop impl | 218-224 | Cleanup on exit |

**Important:** This manages the screenpipe binary as a subprocess!

---

### 3. tracker.rs
**Path:** `/home/user/WorkToJiraEffort/src/tracker.rs`
**Size:** 200 lines

| Function | Lines | Purpose |
|----------|-------|---------|
| `WorkTracker` struct | 12-19 | Main orchestrator |
| `new()` | 22-56 | Constructor, initializes all clients |
| `check_health()` | 58-75 | Verifies all services |
| **`sync()`** | **77-165** | **MAIN POLLING LOOP - KEY INTEGRATION POINT** |
| `consolidate_activities()` | 167-182 | Groups activities by app:window |
| `run()` | 184-198 | Infinite polling loop |

**CRITICAL METHOD: sync() (lines 77-165)**
```
Line 78-83:    Activities fetched from Screenpipe
Line 91:       Activities consolidated (deduplicated)
Line 95-102:   Logging shows what was tracked
Line 105-149:  Jira logging loop
Line 151-161:  Salesforce logging (parallel)
Line 163:      Update last_sync timestamp
```

**Injection Points in sync():**
- **After line 91** - After consolidation, before Jira logging
- **Line 116** - Before find_issue_from_activity() call
- **Line 126** - Before jira.log_work() call

---

### 4. jira.rs
**Path:** `/home/user/WorkToJiraEffort/src/jira.rs`
**Size:** 110 lines

| Function | Lines | Purpose |
|----------|-------|---------|
| `WorklogEntry` struct | 5-11 | What gets sent to Jira |
| `JiraWorklogResponse` struct | 13-17 | Jira response |
| `JiraClient` struct | 19-24 | Jira API client |
| `new()` | 27-34 | Constructor |
| **`log_work()`** | **36-77** | **SENDS TIME TO JIRA** |
| **`find_issue_from_activity()`** | **79-93** | **DETECTS ISSUE KEY (REGEX-BASED)** |
| `health_check()` | 95-108 | Verifies credentials |

**CRITICAL: find_issue_from_activity() (lines 79-93)**
```rust
Line 80-81:    Text assembly from window_title + app_name
Line 84:       Regex pattern: [A-Z]+-\d+ (e.g., PROJ-123)
Line 86-88:    Extract match if found
Line 91:       Return None if not found
```

**CRITICAL: log_work() (lines 36-77)**
```rust
Line 39-44:    Build WorklogEntry
Line 40-43:    Create comment (very simple!)
Line 45-48:    Format timestamp
Line 51-58:    HTTP POST to Jira
Line 60-64:    Error handling
Line 71-75:    Success logging
```

---

### 5. config.rs
**Path:** `/home/user/WorkToJiraEffort/src/config.rs`
**Size:** 115 lines

| Struct | Lines | Purpose |
|--------|-------|---------|
| `Config` | 5-11 | Root config |
| `ScreenpipeConfig` | 13-16 | Screenpipe settings |
| `JiraConfig` | 18-24 | Jira credentials & settings |
| `SalesforceConfig` | 26-35 | Salesforce settings |
| `TrackingConfig` | 37-41 | Polling intervals & filters |

**Key Config Values:**
```toml
[tracking]
poll_interval_secs = 300           # How often to check
min_activity_duration_secs = 60    # Minimum to log

[screenpipe]
url = "http://localhost:3030"

[jira]
url = "https://..."
enabled = true
```

---

### 6. daemon.rs
**Path:** `/home/user/WorkToJiraEffort/src/daemon.rs`
**Size:** 119 lines

| Function | Lines | Purpose |
|----------|-------|---------|
| **`run_daemon()`** | **15-67** | **STARTS HTTP CONTROL API** |
| `DaemonState` struct | 69-72 | State holder |
| `StatusResponse` struct | 74-78 | API response |
| `status_handler()` | 80-86 | GET /status endpoint |
| `issue_override_handler()` | 93-112 | POST /issue endpoint |

**Key Daemon Features:**
- Runs tracker in background (line 41-45)
- Provides HTTP API on port 8787
- Allows issue override via `/issue` endpoint

---

### 7. main.rs
**Path:** `/home/user/WorkToJiraEffort/src/main.rs`
**Size:** 142 lines

| Function | Lines | Purpose |
|----------|-------|---------|
| `Cli` enum | 19-42 | CLI commands |
| **`main()`** | **45-130** | **ENTRY POINT** |
| Commands::Init | 51-59 | Initialize config |
| Commands::Check | 61-82 | Verify connectivity |
| Commands::Start | 84-121 | Begin tracking |
| Commands::Daemon | 122-128 | Start daemon |
| `get_data_dir()` | 133-141 | Get screenpipe data dir |

**Flow:**
```
main() → parse CLI args
  ├─ Init: Create default config
  ├─ Check: Verify all services
  ├─ Start: Begin polling loop
  │   ├─ Start ScreenpipeManager
  │   ├─ Create WorkTracker
  │   ├─ Run sync loop
  │   └─ Stop ScreenpipeManager on Ctrl+C
  └─ Daemon: Start HTTP server
```

---

## Data Flow Map with Line Numbers

```
main.rs (line 84-121) [Commands::Start]
    │
    ├─ screenpipe_manager.rs (line 22-89) [start()]
    │   └─ Spawns Screenpipe subprocess
    │
    ├─ tracker.rs (line 22-56) [WorkTracker::new()]
    │   ├─ screenpipe.rs [ScreenpipeClient::new()]
    │   ├─ jira.rs [JiraClient::new()]
    │   └─ config.rs [Config::load()]
    │
    └─ tracker.rs (line 184-198) [run()]
        └─ LOOP EVERY poll_interval_secs
            │
            ├─ tracker.rs (line 77-165) [sync()]
            │   │
            │   ├─ screenpipe.rs (line 51-111) [get_recent_activities()]
            │   │   └─ HTTP GET localhost:3030/search
            │   │       └─ Returns: Vec<Activity>
            │   │
            │   ├─ tracker.rs (line 167-182) [consolidate_activities()]
            │   │   └─ Group by app:window, sum duration
            │   │
            │   ├─ JIRA LOGGING LOOP (tracker.rs line 105-149)
            │   │   ├─ jira.rs (line 79-93) [find_issue_from_activity()]
            │   │   │   └─ Regex: [A-Z]+-\d+
            │   │   │
            │   │   └─ jira.rs (line 36-77) [log_work()]
            │   │       └─ HTTP POST /rest/api/3/issue/{key}/worklog
            │   │
            │   └─ SALESFORCE LOGGING LOOP (tracker.rs line 151-161)
            │       └─ salesforce.rs [log_time()]
            │
            └─ Sleep for poll_interval_secs
```

---

## Critical Data Transformation Points

### Point 1: Screenpipe JSON → Activity Struct
**File:** `screenpipe.rs`, lines 88-108
```
Input:  ScreenpipeResponse { data: [ScreenpipeSearchEntry, ...] }
Output: Vec<Activity>
Transforms:
- RFC3339 timestamp → DateTime<Utc>
- Sets duration_secs = 60 (hardcoded!)
- Extracts text/content fields
```

### Point 2: Activities → Consolidated Activities
**File:** `tracker.rs`, lines 167-182
```
Input:  Vec<Activity>
Output: Vec<Activity> (deduplicated)
Groups by: format!("{}:{}", app_name, window_title)
Aggregates: Sum duration_secs within groups
```

### Point 3: Activity → Jira Worklog
**File:** `jira.rs`, lines 39-49
```
Input:  Activity
Output: WorklogEntry { comment, time_spent_seconds, started }
Comment: "Auto-tracked: {app_name} - {window_title}"
```

---

## Configuration Loading Order

```
main.rs [main()]
    │
    └─ config.rs [Config::load()]
        ├─ Check if config exists at ~/.config/worktojiraeffort/config.toml
        ├─ If not, create default (config.rs line 43-69)
        ├─ Read TOML file
        └─ Parse into Config struct

Config contains:
- screenpipe.url (default: localhost:3030)
- screenpipe.path
- jira.url, email, api_token, enabled
- salesforce.* settings
- tracking.poll_interval_secs (default: 300)
- tracking.min_activity_duration_secs (default: 60)
```

---

## Summary: Key Code Sections for LLM Integration

| Task | File | Lines | Current Implementation |
|------|------|-------|----------------------|
| Fetch activities | screenpipe.rs | 51-111 | HTTP GET, parse JSON |
| Consolidate | tracker.rs | 167-182 | Group by app:window |
| Detect issue | jira.rs | 79-93 | Simple regex [A-Z]+-\d+ |
| Create comment | jira.rs | 40-43 | Format string |
| Send to Jira | jira.rs | 36-77 | HTTP POST worklog |
| Main loop | tracker.rs | 77-165 | Poll every 5 min |

**WHERE LLM FITS:** Between consolidation (line 182) and Jira logging (line 105)

