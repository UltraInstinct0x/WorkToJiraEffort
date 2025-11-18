# WorkToJiraEffort Architecture Analysis
## Complete Data Flow and Integration Architecture

---

## 1. SCREENPIPE DATA FETCHING/RECEIVING

### Location
**Primary File:** `/home/user/WorkToJiraEffort/src/screenpipe.rs`

### Data Flow
```
Screenpipe API (localhost:3030)
           ↓
    ScreenpipeClient::get_recent_activities()
           ↓
    HTTP GET /search endpoint
           ↓
    Parse JSON response
           ↓
    Convert to Activity structs
```

### Implementation Details
```rust
// Key method in screenpipe.rs (lines 51-111)
pub async fn get_recent_activities(&self, since: DateTime<Utc>) -> Result<Vec<Activity>>

// HTTP Request parameters:
- start_timestamp: Unix timestamp of when to start searching
- end_timestamp: Current time
- limit: 100 (max entries per request)

// Response parsing:
ScreenpipeResponse (wrapper)
  └─ data: Vec<ScreenpipeSearchEntry>
       └─ type: String
       └─ content: ScreenpipeContent
            ├─ frame_id: i64 (optional)
            ├─ text: String (captured text)
            ├─ timestamp: RFC3339 timestamp
            ├─ app_name: String (application name)
            ├─ window_name: String (window title)
            └─ browser_url: String (if browser)
```

### Data Structures
```rust
// Activity struct (what gets passed downstream)
pub struct Activity {
    pub timestamp: DateTime<Utc>,      // When activity occurred
    pub duration_secs: u64,            // Time spent (initially 60s)
    pub window_title: String,          // Window/tab name
    pub app_name: String,              // Application name
    pub description: String,           // Captured text/content
}
```

### Key Processing at Fetch Time
1. **Timestamp Parsing**: RFC3339 → DateTime<Utc>
2. **Default Duration**: All activities initially set to 60 seconds
3. **Fallback Values**: Empty strings for missing fields
4. **Health Checks**: `/health` endpoint verification

---

## 2. JIRA LOGGING LAYER

### Location
**Primary File:** `/home/user/WorkToJiraEffort/src/jira.rs`

### Integration Points
```
WorkTracker receives Activities
       ↓
JiraClient::find_issue_from_activity()  [Issue Detection]
       ├─ Searches window_title & app_name for issue keys
       ├─ Regex pattern: [A-Z]+-\d+ (e.g., PROJ-123)
       └─ Returns: Option<String> (issue key or None)
       ↓
JiraClient::log_work()  [Time Logging]
       ├─ Creates WorklogEntry struct
       ├─ POST to /rest/api/3/issue/{issue_key}/worklog
       ├─ Basic auth: email + API token
       └─ Returns: JiraWorklogResponse
```

### WorklogEntry Structure
```rust
pub struct WorklogEntry {
    pub comment: String,                     // "Auto-tracked: {app} - {title}"
    pub time_spent_seconds: u64,             // Duration from activity
    pub started: String,                     // ISO 8601 format timestamp
}
```

### API Communication
```
POST /rest/api/3/issue/{issue_key}/worklog
Headers:
  - Authorization: Basic <base64(email:token)>
  - Content-Type: application/json
Body:
  {
    "comment": "Auto-tracked: Chrome - PROJ-123 Issue Title",
    "timeSpentSeconds": 3600,
    "started": "2025-11-18T14:30:00.000+00:00"
  }
```

### Health Check
- Endpoint: `/rest/api/3/myself`
- Purpose: Verify credentials and connectivity
- Authentication: Same basic auth as worklog

---

## 3. DATA FLOW BETWEEN SCREENPIPE AND JIRA

### Complete Workflow Sequence

```
1. INITIALIZATION (main.rs, daemon.rs)
   ├─ Load configuration (config.toml)
   ├─ Start ScreenpipeManager subprocess
   │  └─ Manages Screenpipe binary lifecycle
   ├─ Initialize WorkTracker
   │  ├─ Create ScreenpipeClient
   │  ├─ Create JiraClient (if enabled)
   │  └─ Create SalesforceClient (if enabled)
   └─ Health checks all services

2. POLLING LOOP (tracker.rs, sync() method)
   ┌─────────────────────────────────────────┐
   │ EVERY poll_interval_secs (default: 300) │
   └─────────────────────────────────────────┘
   
   ├─ Get recent activities
   │  └─ screenpipe.get_recent_activities(since: DateTime)
   │     └─ HTTP GET to localhost:3030/search
   │        └─ Returns: Vec<Activity>
   │           ├─ timestamp
   │           ├─ duration_secs (hardcoded 60s)
   │           ├─ window_title
   │           ├─ app_name
   │           └─ description
   │
   ├─ CONSOLIDATE activities (by app:window)
   │  ├─ Group by (app_name, window_title)
   │  └─ Sum duration_secs within groups
   │     └─ Example:
   │        Input:  [Chrome/PROJ-123 (60s), Chrome/PROJ-123 (60s)]
   │        Output: [Chrome/PROJ-123 (120s)]
   │
   ├─ FILTER by minimum duration
   │  ├─ min_activity_duration_secs from config (default: 60)
   │  └─ Skip if duration < minimum
   │
   ├─ LOG TO JIRA (if configured)
   │  │
   │  └─ For each consolidated activity:
   │     │
   │     ├─ Option 1: Use issue_override (if set via daemon API)
   │     │  └─ jira.log_work(issue_key, activity)
   │     │
   │     ├─ Option 2: Auto-detect from activity
   │     │  ├─ jira.find_issue_from_activity(activity)
   │     │  │  └─ Regex search: [A-Z]+-\d+
   │     │  │  └─ Searches: window_title + app_name
   │     │  │
   │     │  └─ If found:
   │     │     └─ jira.log_work(detected_issue_key, activity)
   │     │
   │     └─ If no issue found:
   │        └─ Log warning: "Skipped (no Jira issue found)"
   │
   ├─ LOG TO SALESFORCE (if configured, parallel to Jira)
   │  └─ salesforce.log_time(activity)
   │     └─ Creates TimeEntry__c record
   │
   └─ Update last_sync timestamp

3. SHUTDOWN
   └─ ScreenpipeManager stops Screenpipe subprocess
```

---

## 4. DATA PROCESSING AND FILTERING LOGIC

### Current Processing Pipeline

```
Raw Screenpipe Data
    ↓
[STEP 1] Timestamp Parsing
├─ RFC3339 string → DateTime<Utc>
├─ Fallback: Utc::now() if invalid
└─ Result: normalized DateTime<Utc>

    ↓
[STEP 2] Activity Consolidation
├─ Key: "{app_name}:{window_title}"
├─ Aggregation: SUM duration_secs
└─ Result: deduplicated activities

    ↓
[STEP 3] Minimum Duration Filter
├─ Check: activity.duration_secs >= min_activity_duration_secs
├─ Skip: too short activities
└─ Result: filtered activities

    ↓
[STEP 4] Jira Issue Detection
├─ Regex: [A-Z]+-\d+ on "{window_title} {app_name}"
├─ Override: use issue_override if set
└─ Result: issue_key or None

    ↓
[STEP 5] Logging
├─ Skip: if no issue key (warning logged)
├─ Log: if issue key found
└─ Result: worklog entry in Jira
```

### Configuration Parameters

```toml
[tracking]
poll_interval_secs = 300        # How often to check Screenpipe (5 min)
min_activity_duration_secs = 60 # Minimum 1 minute to be logged

[jira]
url = "https://company.atlassian.net"
email = "user@company.com"
api_token = "your-api-token"
enabled = true

[screenpipe]
url = "http://localhost:3030"   # Default embedded instance

[salesforce]
# ... OAuth credentials ...
enabled = false                  # Can be disabled
```

### Current Limitations/Observations

1. **No Text Analysis**: Activity descriptions are not processed
2. **Simple Issue Detection**: Only regex pattern matching on titles
3. **No Intelligent Categorization**: Activities not classified by work type
4. **No Context Enrichment**: Activities logged with minimal metadata
5. **Hard-coded Duration**: Initial 60s, only changes via consolidation

---

## 5. MAIN COMPONENTS AND THEIR RESPONSIBILITIES

### Component Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    main.rs (Entry Point)                    │
│  ├─ Commands: Init, Check, Start, Daemon                   │
│  └─ Manages: CLI parsing, ScreenpipeManager lifecycle       │
└──────────────────────────┬──────────────────────────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
    ┌───▼────────┐  ┌──────▼──────┐  ┌──────▼──────┐
    │config.rs   │  │daemon.rs    │  │tracker.rs   │
    ├────────────┤  ├─────────────┤  ├─────────────┤
    │• Load TOML │  │• HTTP server│  │• Main logic │
    │• Parse     │  │• /status    │  │• Polling    │
    │• Defaults  │  │• /issue     │  │• Consol.    │
    │• Save      │  │• Port 8787  │  │• Detection  │
    └────────────┘  └─────────────┘  └────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
    ┌───▼──────────┐   ┌───▼──────┐   ┌──────▼─────┐
    │screenpipe.rs │   │jira.rs   │   │salesforce  │
    ├──────────────┤   ├──────────┤   │.rs         │
    │• HTTP Client │   │• Worklog │   ├────────────┤
    │• Parse JSON  │   │• Issue   │   │• Auth      │
    │• Activities  │   │  detect  │   │• Time Entry│
    │• Health chk  │   │• Health  │   │• Refresh   │
    └──────────────┘   └──────────┘   └────────────┘
             │                │              │
             └────────────────┼──────────────┘
                              │
        ┌─────────────────────┼──────────────────┐
        │                     │                  │
    ┌───▼────────────┐   ┌────▼─────┐   ┌──────▼──┐
    │screenpipe_     │   │localhost: │   │External │
    │manager.rs      │   │3030       │   │APIs     │
    ├────────────────┤   ├───────────┤   ├─────────┤
    │• Subprocess    │   │Activities │   │Jira     │
    │  mgmt          │   │Data       │   │SF       │
    │• Binary find   │   │           │   │         │
    │• Start/stop    │   │           │   │         │
    └────────────────┘   └───────────┘   └─────────┘
```

### Component Responsibilities

| Component | Purpose | Key Methods |
|-----------|---------|------------|
| **main.rs** | CLI entry point | main(), get_data_dir() |
| **config.rs** | Config management | load(), save(), config_path() |
| **tracker.rs** | Core orchestration | sync(), run(), consolidate_activities() |
| **screenpipe.rs** | Screenpipe API | get_recent_activities(), health_check() |
| **screenpipe_manager.rs** | Subprocess lifecycle | start(), stop(), find_binary() |
| **jira.rs** | Jira integration | log_work(), find_issue_from_activity() |
| **salesforce.rs** | Salesforce integration | log_time(), authenticate() |
| **daemon.rs** | HTTP control API | run_daemon(), status_handler() |

---

## 6. RECOMMENDED INTEGRATION POINTS FOR LLM ANALYSIS LAYER

### Where LLM Analysis Could Add Value

#### Option 1: Activity Description Analysis
**Location:** `tracker.rs:sync()` method, after consolidation

```
Current Flow:
Activities (with descriptions) → Jira Issue Detection (regex) → Log

Enhanced Flow:
Activities (with descriptions) → LLM Analysis
├─ Classify activity type (meeting, coding, debugging, etc.)
├─ Generate summary from captured text
├─ Extract key topics/projects
├─ Suggest issue category
└─ Enhanced issue detection
   ↓
→ Better issue matching
→ Richer worklog comments
→ Categorized time tracking
```

#### Option 2: Smart Issue Detection
**Location:** `jira.rs:find_issue_from_activity()`

```
Current: Regex pattern [A-Z]+-\d+

Enhanced with LLM:
├─ Extract entities from activity description
├─ Understand work context from text
├─ Map to likely Jira projects
├─ Rank multiple candidates
└─ Higher accuracy issue detection
```

#### Option 3: Activity Enrichment
**Location:** Between screenpipe fetching and jira logging

```
Raw Activity Data
├─ Timestamp
├─ Duration
├─ App name
├─ Window title
└─ Captured text

LLM Enhancement:
├─ Generate meaningful summary
├─ Extract technologies mentioned
├─ Identify blockers/issues
├─ Tag with work category
└─ Suggest time allocation
```

#### Option 4: Consolidated Report Generation
**Location:** New module or daemon endpoint

```
Before logging each activity individually:
├─ Collect all activities for period
├─ Use LLM to generate summary
├─ Identify patterns and trends
├─ Suggest issue consolidation
└─ Optimize Jira worklog entries
```

---

## 7. DATA STRUCTURE TRANSFORMATIONS

### Complete Data Journey

```
Screenpipe Raw JSON
├─ type: "ocr"
├─ content:
│  ├─ timestamp: "2025-11-18T14:30:00Z"
│  ├─ app_name: "Google Chrome"
│  ├─ window_name: "PROJ-123: Implement feature"
│  └─ text: "[extracted text from screen]"
└─ [multiple entries]

↓ [parse & aggregate]

Vec<Activity>
├─ timestamp: DateTime<Utc>
├─ duration_secs: 60
├─ app_name: "Google Chrome"
├─ window_title: "PROJ-123: Implement feature"
└─ description: "[extracted text]"

↓ [consolidate]

Vec<Activity> (deduplicated)
└─ [same app:window grouped with summed duration]

↓ [filter & detect]

For each Activity:
├─ Detected issue_key: "PROJ-123"
├─ Worklog comment: "Auto-tracked: Google Chrome - PROJ-123: Implement feature"
├─ Time spent: 60 seconds
└─ Timestamp: "2025-11-18T14:30:00.000Z"

↓ [log to Jira]

POST /rest/api/3/issue/PROJ-123/worklog
→ JiraWorklogResponse { id: "..." }
```

---

## Summary: Complete Architecture

The application follows a **pipeline architecture**:

1. **Data Acquisition**: Screenpipe polls screen activity
2. **Data Processing**: Activities consolidated and filtered
3. **Intelligence Layer**: Issue detection via regex pattern matching
4. **Data Enrichment**: Convert to Jira/Salesforce format
5. **Integration**: Push to external systems

**Key Observation**: The intelligence layer (issue detection) is very simple - just regex matching. This is **where an LLM would have maximum impact**, transforming raw screen data into contextual work assignments with semantic understanding.

