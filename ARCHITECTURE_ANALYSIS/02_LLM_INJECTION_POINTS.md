# LLM Integration Points - Detailed Code Map

## Current Code Flow with Injection Points

### Starting Point: tracker.rs - sync() method (line 77-165)

```rust
pub async fn sync(&mut self) -> Result<()> {
    // STEP 1: Fetch from Screenpipe
    let activities = self.screenpipe
        .get_recent_activities(self.last_sync)  // Lines 80-83
        .await?;
    
    // STEP 2: Consolidate (deduplicate by app:window)
    let consolidated = self.consolidate_activities(&activities);  // Line 91
    
    // *** INJECTION POINT 1: HERE ***
    // After consolidation, before logging
    // Would be perfect place to run LLM on descriptions
    
    // STEP 3: Log to Jira
    if let Some(jira) = &self.jira {
        for activity in &consolidated {
            if activity.duration_secs >= self.config.tracking.min_activity_duration_secs {
                
                // *** INJECTION POINT 2: HERE ***
                // Before calling find_issue_from_activity
                // Could pass description to LLM for better detection
                
                let target_issue = if let Some(issue_key) = &issue_override {
                    Some(issue_key.clone())
                } else {
                    match jira.find_issue_from_activity(activity).await {
                        // This uses simple regex [A-Z]+-\d+
                        // COULD BE ENHANCED WITH LLM
                        Ok(result) => result,
                        Err(err) => None
                    }
                };
                
                if let Some(issue_key) = target_issue {
                    // *** INJECTION POINT 3: HERE ***
                    // Before logging, could enhance comment with LLM analysis
                    jira.log_work(&issue_key, activity).await
                } else {
                    log::warn!("Skipped (no Jira issue found)");
                }
            }
        }
    }
}
```

---

## Detailed Injection Point Analysis

### INJECTION POINT 1: Activity Enrichment (Post-Consolidation)
**File:** `/home/user/WorkToJiraEffort/src/tracker.rs` (after line 91)
**Trigger:** After consolidate_activities()
**Input Data:**
```rust
Vec<Activity> {
    timestamp: DateTime<Utc>,
    duration_secs: u64,
    window_title: String,      // e.g., "PROJ-123: Implement payment"
    app_name: String,           // e.g., "IntelliJ IDEA"
    description: String,        // Raw captured text
}
```

**What LLM Should Do:**
```
For each activity, analyze:
- Captured text/description
- Window title context
- Application name context

Output enrichment:
{
    activity: Activity,
    llm_analysis: {
        work_type: "coding" | "meeting" | "research" | "documentation",
        confidence: 0.0..1.0,
        key_topics: Vec<String>,      // ["authentication", "payment", "API"]
        suggested_projects: Vec<(String, f32)>,  // [("PROJ", 0.95), ("TEAM", 0.3)]
        summary: String,              // Human-readable summary
    }
}
```

**Current Code Location:**
- File: `/home/user/WorkToJiraEffort/src/tracker.rs`
- Lines: 167-182 (consolidate_activities method)
- Would insert new method call here

**Integration Pattern:**
```rust
// After consolidation
let enriched = self.enrich_activities_with_llm(&consolidated).await?;
// enriched: Vec<(Activity, LLMAnalysis)>
```

---

### INJECTION POINT 2: Smart Issue Detection (Before find_issue_from_activity)
**File:** `/home/user/WorkToJiraEffort/src/jira.rs` (enhancement to lines 79-93)
**Current Code:**
```rust
pub async fn find_issue_from_activity(&self, activity: &Activity) -> Result<Option<String>> {
    // Simple heuristic: look for Jira issue keys (e.g., PROJ-123)
    let text = format!("{} {}", activity.window_title, activity.app_name);
    
    // Regex pattern for Jira issue keys
    let issue_key_regex = regex::Regex::new(r"([A-Z]+-\d+)").unwrap();
    
    if let Some(captures) = issue_key_regex.captures(&text) {
        if let Some(issue_key) = captures.get(1) {
            return Ok(Some(issue_key.as_str().to_string()));
        }
    }
    
    Ok(None)
}
```

**Enhanced Version Would Be:**
```rust
pub async fn find_issue_from_activity(&self, activity: &Activity) -> Result<Option<String>> {
    // Try regex first (fast path)
    let text = format!("{} {}", activity.window_title, activity.app_name);
    let issue_key_regex = regex::Regex::new(r"([A-Z]+-\d+)").unwrap();
    
    if let Some(captures) = issue_key_regex.captures(&text) {
        if let Some(issue_key) = captures.get(1) {
            return Ok(Some(issue_key.as_str().to_string()));
        }
    }
    
    // If regex fails, try LLM-based detection
    // This uses the activity's description and context
    if let Some(suggested_project) = self.llm_suggest_issue(activity).await? {
        return Ok(Some(suggested_project));
    }
    
    Ok(None)
}
```

**Data Flowing In:**
```rust
Activity {
    window_title: "Meeting about payment gateway",  // No obvious issue key
    description: "Discussed PROJ-456 API integration needs",  // Contains issue!
    app_name: "Google Meet"
}
```

**What LLM Should Do:**
- Read: window_title + app_name + description
- Identify: Project context ("PROJ-456" hidden in description)
- Return: Issue key or suggestions

---

### INJECTION POINT 3: Worklog Comment Enhancement (Before log_work)
**File:** `/home/user/WorkToJiraEffort/src/jira.rs` (lines 36-49)
**Current Code:**
```rust
pub async fn log_work(&self, issue_key: &str, activity: &Activity) -> Result<()> {
    let url = format!("{}/rest/api/3/issue/{}/worklog", self.base_url, issue_key);
    
    let worklog = WorklogEntry {
        comment: format!(
            "Auto-tracked: {} - {}",
            activity.app_name, 
            activity.window_title
        ),
        time_spent_seconds: activity.duration_secs,
        started: activity.timestamp
            .format("%Y-%m-%dT%H:%M:%S%.3f%z")
            .to_string(),
    };
    
    // POST to Jira...
}
```

**Enhanced Version:**
```rust
pub async fn log_work(&self, issue_key: &str, activity: &Activity) -> Result<()> {
    let url = format!("{}/rest/api/3/issue/{}/worklog", self.base_url, issue_key);
    
    // Generate richer comment using LLM analysis
    let comment = self.generate_enhanced_comment(activity).await?;
    // Instead of: "Auto-tracked: IntelliJ - PROJ-123: Feature"
    // Becomes: "Development work: Implemented OAuth integration, fixed auth flow issues"
    
    let worklog = WorklogEntry {
        comment,  // Now enriched by LLM
        time_spent_seconds: activity.duration_secs,
        started: activity.timestamp
            .format("%Y-%m-%dT%H:%M:%S%.3f%z")
            .to_string(),
    };
    
    // POST to Jira...
}
```

**Input Activity:**
```rust
Activity {
    description: "Reviewed PR #456, discussing auth flow changes, merged feature branch",
    window_title: "IntelliJ IDEA - PROJ-123: Auth Service",
    app_name: "IntelliJ IDEA",
    duration_secs: 3600,  // 1 hour
}
```

**Current Comment:**
```
Auto-tracked: IntelliJ IDEA - IntelliJ IDEA - PROJ-123: Auth Service
```

**LLM-Enhanced Comment:**
```
Development: Reviewed and merged PR #456 for OAuth integration, 
improved auth flow security. ~1h coding + review.
```

---

## OPTIONAL: INJECTION POINT 4 (Advanced)
**Location:** New async report generation (not in current code)
**Use Case:** Batch processing before any logging

```rust
pub async fn batch_analyze_activities(&self, activities: &[Activity]) -> Result<BatchAnalysis> {
    // Instead of processing each activity individually
    // Collect a batch and send to LLM once
    
    let batch_context = ActivityBatch {
        time_period: (start_time, end_time),
        total_duration_secs: activities.iter().map(|a| a.duration_secs).sum(),
        activities: activities.to_vec(),
    };
    
    // Call LLM to:
    // 1. Identify overall themes/projects
    // 2. Suggest issue consolidation
    // 3. Generate summary for status updates
    // 4. Detect patterns in work
    
    Ok(batch_analysis)
}
```

---

## Data Availability at Each Injection Point

| Injection Point | Available Data | LLM Capability | Effort |
|---|---|---|---|
| **1. Post-Consolidation** | app_name, window_title, description, timestamp, duration | Full activity context | Medium |
| **2. Before Issue Detection** | Same as above | Entity extraction, context understanding | Medium |
| **3. Before Log Upload** | activity + detected_issue_key | Contextual summarization | Low |
| **4. Batch Analysis** | Multiple activities, time range | Pattern recognition, high-level insights | High |

---

## Recommended Implementation Strategy

### Phase 1: Issue Detection Enhancement (Lowest Risk)
**Start Here:** Injection Point 2
```rust
fn find_issue_from_activity_with_llm(activity: &Activity) -> Result<Option<String>>
```
- Keep regex fallback
- Only use LLM if regex fails
- Caches results to avoid re-analyzing
- Cost: ~1 extra API call per unmatched activity

### Phase 2: Comment Enrichment
**Next:** Injection Point 3
```rust
fn generate_enhanced_comment(activity: &Activity) -> Result<String>
```
- Transforms "Auto-tracked: Chrome - PROJ-123" 
- Into: "Implementation: Fixed login flow bugs, merged feature PR"
- Cost: 1 API call per logged activity

### Phase 3: Activity Enrichment & Categorization
**Final:** Injection Point 1
```rust
fn enrich_activities_with_llm(activities: &[Activity]) -> Result<Vec<LLMEnrichedActivity>>
```
- Classify work types
- Extract topics/keywords
- Group related activities
- Cost: ~1 API call per batch

---

## LLM Integration Technical Considerations

### Recommended Approach
1. **New Module:** `src/llm_analyzer.rs`
2. **Trait-based Design:** Allow swappable LLM backends (OpenAI, Claude, local)
3. **Async Operations:** Use tokio for non-blocking LLM calls
4. **Error Handling:** Graceful degradation if LLM unavailable
5. **Caching:** Store analyzed activities to avoid re-processing
6. **Rate Limiting:** Batch requests to minimize API calls

### Code Structure
```rust
// src/llm_analyzer.rs
pub trait LLMAnalyzer {
    async fn analyze_activity(&self, activity: &Activity) -> Result<ActivityAnalysis>;
    async fn suggest_issue_key(&self, activity: &Activity) -> Result<Option<String>>;
    async fn generate_summary(&self, activity: &Activity) -> Result<String>;
}

pub struct OpenAIAnalyzer {
    client: OpenAI,
    model: String,
    api_key: String,
}

impl LLMAnalyzer for OpenAIAnalyzer {
    // Implementation...
}
```

### Configuration Addition
```toml
[llm]
enabled = true
provider = "openai"  # or "claude", "anthropic"
model = "gpt-4-turbo"
api_key = "sk-..."
batch_size = 5  # Process N activities at once
cache_results = true
timeout_secs = 30
```

