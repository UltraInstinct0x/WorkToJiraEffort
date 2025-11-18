# WorkToJiraEffort Architecture - Executive Summary

## Quick Answer: How It Works

The application follows a simple **polling pipeline**:

```
1. Screenpipe captures screen activity (what app/window, timestamps, text)
2. Every 5 minutes, the app queries Screenpipe API 
3. Activities are deduplicated (grouped by app:window)
4. For each activity:
   a. Search window title for Jira issue key (regex: PROJ-123)
   b. If found, log time to Jira via REST API
   c. If found, log time to Salesforce via REST API
5. Repeat step 2
```

---

## The 5 Key Data Transformations

### 1. Raw Screenpipe JSON → Activity Objects
- **Where:** `screenpipe.rs:51-111`
- **What:** HTTP GET to localhost:3030/search, parse JSON response
- **Input:** `{"data": [{"type": "ocr", "content": {...}}]}`
- **Output:** `Activity { timestamp, duration_secs: 60, window_title, app_name, description }`
- **Limitation:** All durations default to 60 seconds initially

### 2. Activities → Consolidated Activities
- **Where:** `tracker.rs:167-182`
- **What:** Group by `"{app_name}:{window_title}"`, sum durations
- **Why:** Reduce noise from rapid window switches
- **Example:** 
  - Input: `[Chrome/PROJ-123 (60s), Chrome/PROJ-123 (60s)]`
  - Output: `[Chrome/PROJ-123 (120s)]`

### 3. Activities → Filtered Activities
- **Where:** `tracker.rs:112`
- **Rule:** Only log activities >= 60 seconds (configurable)
- **Rationale:** Ignore very brief activities

### 4. Activities → Issue Detection
- **Where:** `jira.rs:79-93`
- **Method:** Regex `[A-Z]+-\d+` on window_title + app_name
- **Examples:**
  - ✓ `"PROJ-456 Implement Auth"` → Detects `PROJ-456`
  - ✓ `"Bug TEAM-789: Fix UI"` → Detects `TEAM-789`
  - ✗ `"Meeting with team"` → Detects nothing
  - ✗ `"Working on payment"` (issue hidden in description) → Detects nothing

### 5. Activities → Jira Worklog Entry
- **Where:** `jira.rs:36-77`
- **What:** POST to `/rest/api/3/issue/{issue_key}/worklog`
- **Data:**
  ```json
  {
    "comment": "Auto-tracked: Chrome - PROJ-123: Feature Title",
    "timeSpentSeconds": 3600,
    "started": "2025-11-18T14:30:00.000Z"
  }
  ```

---

## Architecture Diagram

```
SCREENPIPE LAYER (localhost:3030)
    ↓ [HTTP GET /search]
FETCH LAYER (screenpipe.rs)
    ↓ [Parse JSON, normalize timestamps]
CONSOLIDATION LAYER (tracker.rs)
    ↓ [Group by app:window, sum durations]
FILTERING LAYER (tracker.rs)
    ↓ [Skip if duration < 60s]
DETECTION LAYER (jira.rs) ⚠️ SIMPLE REGEX
    ├─ Issue Key: PROJ-456 (if found in title)
    └─ None (if not found)
ENRICHMENT LAYER (currently minimal)
    ├─ Comment: "Auto-tracked: Chrome - PROJ-123"
    └─ No semantic analysis
LOGGING LAYER (jira.rs + salesforce.rs)
    ├─ HTTP POST to Jira REST API
    └─ HTTP POST to Salesforce REST API
```

---

## Component Responsibilities

| Component | Role | Key Method |
|-----------|------|-----------|
| **screenpipe.rs** | Fetch raw activity data | `get_recent_activities(since)` |
| **screenpipe_manager.rs** | Manage subprocess lifecycle | `start()`, `stop()` |
| **tracker.rs** | Orchestrate entire flow | `sync()`, `consolidate_activities()` |
| **jira.rs** | Issue detection + Jira API | `find_issue_from_activity()`, `log_work()` |
| **config.rs** | Load/save configuration | `load()`, `save()` |
| **daemon.rs** | HTTP control API | `run_daemon()` |
| **main.rs** | CLI entry point | `main()` |

---

## Current Limitations (Where LLM Adds Value)

### Problem 1: Simple Issue Detection
**Current:** Regex only looks for issue keys in window title/app name
```
Window: "Meeting about payment gateway"
Description: "Discussed implementing PROJ-456 API integration"
Result: ✗ Issue NOT detected (key is in description, not title)
```

**LLM Solution:** Parse description field to find context clues

### Problem 2: No Activity Classification
**Current:** All activities treated equally
```
1 hour Chrome: "PROJ-123"  → "1h coding"?
1 hour Chrome: "PROJ-123"  → "1h research"?
1 hour Chrome: "PROJ-123"  → "1h meetings"?
(Can't tell the difference!)
```

**LLM Solution:** Analyze captured text to classify work type

### Problem 3: Generic Comments
**Current:**
```
"Auto-tracked: IntelliJ IDEA - PROJ-123: Feature"
```

**LLM-Enhanced:**
```
"Development: Implemented OAuth integration, 
improved error handling. ~1h coding + review."
```

### Problem 4: No Text Analysis
**Current:** Captured text/description is ignored
```
Activity.description = "Reviewed PR #456, merged OAuth changes"
         ↓
         [UNUSED - not analyzed]
```

**LLM Solution:** Extract key information from captured text

### Problem 5: No Activity Enrichment
**Current:** Activities not tagged with metadata
```
Activity {
    window_title: "Chrome - PROJ-456",
    description: "[raw screen text]",
    app_name: "Chrome",
    duration_secs: 3600
}
```

**LLM-Enhanced:**
```
Activity {
    window_title: "Chrome - PROJ-456",
    description: "[raw screen text]",
    app_name: "Chrome",
    duration_secs: 3600,
    
    // LLM additions:
    work_type: "development",
    key_topics: ["OAuth", "authentication"],
    summary: "Implemented OAuth integration flow",
    confidence: 0.95,
    technologies: ["JavaScript", "Node.js"],
}
```

---

## Recommended LLM Integration Strategy

### Phase 1: Smart Issue Detection (LOW RISK)
**Effort:** 2-3 days
**Impact:** Catch issues hidden in descriptions
**Where:** `jira.rs:find_issue_from_activity()`
**How:** 
```
1. Try regex (existing)
2. If no match, ask LLM: "Find Jira issue key in this context"
3. Return best match or None
```
**Cost:** ~1 API call per activity without obvious issue

### Phase 2: Comment Enrichment (LOW RISK)
**Effort:** 1-2 days
**Impact:** Richer Jira worklogs
**Where:** `jira.rs:log_work()`
**How:**
```
1. Take Activity.description (captured text)
2. Ask LLM: "Summarize what the user was working on"
3. Use result as worklog comment
```
**Cost:** ~1 API call per logged activity

### Phase 3: Activity Enrichment (MEDIUM RISK)
**Effort:** 3-5 days
**Impact:** Better categorization and reporting
**Where:** `tracker.rs:sync()` after consolidation
**How:**
```
1. For each activity, ask LLM to classify
2. Store classification with activity
3. Use for filtering/reporting
```
**Cost:** ~1 API call per activity

### Phase 4: Batch Analysis (ADVANCED)
**Effort:** 5-7 days
**Impact:** Pattern recognition, weekly summaries
**Where:** New module or daemon endpoint
**How:**
```
1. Collect activities for time period
2. Ask LLM: "Generate work summary for this period"
3. Show patterns, time allocation, highlights
```
**Cost:** ~1 API call per batch

---

## What Data Is Available at Decision Points

```
At SCREENPIPE FETCH (screenpipe.rs:51-111):
├─ Timestamp (RFC3339 string)
├─ App name (e.g., "Chrome", "IntelliJ")
├─ Window title (e.g., "PROJ-123: Feature X")
└─ Raw text (e.g., "Reviewing PR #456")

At CONSOLIDATION (tracker.rs:167-182):
├─ Vec<Activity> with normalized timestamps
├─ Consolidated by app:window
└─ Summed durations

At DETECTION (jira.rs:79-93):
├─ Full Activity struct
├─ Window title
├─ App name
└─ Description (CURRENTLY UNUSED!)

At LOGGING (jira.rs:36-77):
├─ Activity
├─ Detected issue_key
└─ About to generate comment
```

**Key Insight:** The `Activity.description` field contains valuable captured text that is **completely unused** in issue detection or comment generation!

---

## Configuration for LLM

Add to `config.toml`:

```toml
[llm]
enabled = true
provider = "anthropic"  # or "openai", "local"
api_key = "sk-ant-..."
model = "claude-3-5-sonnet-20241022"

# Options to tune behavior
batch_size = 5              # Process N activities at once
cache_results = true        # Don't re-analyze same activities
timeout_secs = 30           # Max wait for LLM response
enable_issue_detection = true
enable_comment_enrichment = true
enable_activity_classification = false  # Future
enable_batch_analysis = false           # Future
```

---

## Key Files to Modify

For Phase 1 (Smart Issue Detection):
1. **jira.rs** - Enhance `find_issue_from_activity()`
2. **config.rs** - Add LLM config section
3. **tracker.rs** - Pass description to issue detection
4. **new file: llm.rs** - LLM API client

For Phase 2 (Comment Enrichment):
1. **jira.rs** - Enhance `log_work()`
2. **new file: llm.rs** - Add `generate_summary()` method

For Phase 3 (Activity Enrichment):
1. **tracker.rs** - Add enrichment step after consolidation
2. **screenpipe.rs** - Extend Activity struct with analysis
3. **llm.rs** - Add `classify_activity()` method

---

## Success Criteria

### Phase 1
- [ ] Issues in descriptions are detected 80% of the time
- [ ] No regex matches are broken
- [ ] LLM calls fail gracefully (fall back to regex)
- [ ] Performance: detection adds <1s per batch

### Phase 2
- [ ] Comments are human-readable
- [ ] Comments are <280 characters (Jira constraint)
- [ ] Context is preserved (what app, what project)
- [ ] No PII leakage in comments

### Phase 3
- [ ] Activities classified into 5+ categories
- [ ] Classification accuracy >80%
- [ ] Can filter by work type for reporting
- [ ] Optional (can be disabled)

---

## Implementation Notes

1. **Error Handling:** If LLM fails, gracefully degrade:
   - Issue detection: use regex only
   - Comment enrichment: use default comment
   - Activity classification: leave as unclassified

2. **Performance:** 
   - Batch requests to minimize API calls
   - Cache results to avoid re-analyzing
   - Keep synchronous Jira logging path fast

3. **Privacy:**
   - Don't send raw screen text to LLM (privacy risk)
   - Send only: app name, window title, description summary
   - Consider local LLM option for sensitive data

4. **Testing:**
   - Mock LLM for unit tests
   - Add integration tests with real LLM
   - Test graceful degradation when LLM unavailable

