# Annotated Code Flow with LLM Integration Points

This document shows the actual code flow with exact locations where LLM analysis should be injected.

## Main Loop Flow (tracker.rs:184-198)

```rust
pub async fn run(&mut self, interval_secs: u64) -> Result<()> {
    loop {
        match self.sync().await {          // ◄── MAIN PROCESSING
            Ok(_) => {},
            Err(e) => log::error!("Sync failed: {:#}", e),
        }
        tokio::time::sleep(Duration::from_secs(interval_secs)).await;  // ◄── SLEEP 300s
    }
}
```

## Detailed sync() Method (tracker.rs:77-165)

```rust
pub async fn sync(&mut self) -> Result<()> {
    // ═════════════════════════════════════════════════════════════════
    // STEP 1: FETCH SCREENPIPE DATA
    // ═════════════════════════════════════════════════════════════════
    let activities = self.screenpipe
        .get_recent_activities(self.last_sync)        // ◄── screenpipe.rs:51-111
        .await?;                                       // Returns: Vec<Activity>
    
    log::info!("Found {} activities", activities.len());
    
    if activities.is_empty() {
        return Ok(());
    }
    
    // ═════════════════════════════════════════════════════════════════
    // STEP 2: CONSOLIDATE (GROUP BY APP:WINDOW)
    // ═════════════════════════════════════════════════════════════════
    let consolidated = self.consolidate_activities(&activities);   // ◄── Line 167-182
    log::info!("Consolidated into {} entries", consolidated.len());
    
    // ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
    // ┃  INJECTION POINT 1: HERE (after consolidation)          ┃
    // ┃  Add LLM analysis to enrich activities before logging   ┃
    // ┃  let enriched = self.enrich_with_llm(&consolidated)?;   ┃
    // ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
    
    // ═════════════════════════════════════════════════════════════════
    // STEP 3: SHOW WHAT WAS TRACKED
    // ═════════════════════════════════════════════════════════════════
    for activity in &consolidated {
        log::info!(
            "  {} - {} ({} mins)",
            activity.app_name,
            activity.window_title,
            activity.duration_secs / 60
        );
    }
    
    // ═════════════════════════════════════════════════════════════════
    // STEP 4: LOG TO JIRA
    // ═════════════════════════════════════════════════════════════════
    if let Some(jira) = &self.jira {
        let issue_override = {
            let guard = self.issue_override.read().await;
            guard.clone()
        };
        
        for activity in &consolidated {
            // ═════════════════════════════════════════════════════════
            // STEP 4A: FILTER BY MINIMUM DURATION
            // ═════════════════════════════════════════════════════════
            if activity.duration_secs >= self.config.tracking.min_activity_duration_secs {
                
                // ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
                // ┃  INJECTION POINT 2: HERE (before issue detection) ┃
                // ┃  Could enhance issue detection with LLM          ┃
                // ┃  let llm_detection = llm.suggest_issue(activity)?;
                // ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
                
                // ═════════════════════════════════════════════════════
                // STEP 4B: DETECT JIRA ISSUE (REGEX-BASED)
                // ═════════════════════════════════════════════════════
                let target_issue = if let Some(issue_key) = &issue_override {
                    Some(issue_key.clone())
                } else {
                    match jira.find_issue_from_activity(activity).await {
                        // ◄── jira.rs:79-93 (REGEX ONLY!)
                        // CURRENTLY IGNORES activity.description!
                        Ok(result) => result,
                        Err(err) => {
                            log::error!("Failed to detect Jira issue: {}", err);
                            None
                        }
                    }
                };
                
                if let Some(issue_key) = target_issue {
                    // ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
                    // ┃  INJECTION POINT 3: HERE (before logging)  ┃
                    // ┃  Enhance comment with LLM analysis        ┃
                    // ┃  let comment = llm.summarize(activity)?;  ┃
                    // ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
                    
                    // ═════════════════════════════════════════════
                    // STEP 4C: LOG TO JIRA
                    // ═════════════════════════════════════════════
                    match jira.log_work(&issue_key, activity).await {
                        // ◄── jira.rs:36-77 (HTTP POST)
                        Ok(_) => log::info!("Successfully logged to Jira: {}", issue_key),
                        Err(e) => log::error!("Failed to log to Jira: {}", e),
                    }
                } else {
                    // No issue found - would benefit from LLM!
                    log::warn!(
                        "Skipped (no Jira issue found): {} - {}",
                        activity.app_name,
                        activity.window_title,
                    );
                }
            } else {
                // Too short - skip
                log::debug!(
                    "Skipped (too short): {} - {} ({} secs)",
                    activity.app_name,
                    activity.window_title,
                    activity.duration_secs
                );
            }
        }
    }
    
    // ═════════════════════════════════════════════════════════════════
    // STEP 5: LOG TO SALESFORCE (if configured)
    // ═════════════════════════════════════════════════════════════════
    if let Some(salesforce) = &mut self.salesforce {
        for activity in &consolidated {
            if activity.duration_secs >= self.config.tracking.min_activity_duration_secs {
                match salesforce.log_time(activity).await {
                    Ok(_) => log::info!("Successfully logged to Salesforce"),
                    Err(e) => log::error!("Failed to log to Salesforce: {}", e),
                }
            }
        }
    }
    
    // ═════════════════════════════════════════════════════════════════
    // STEP 6: UPDATE SYNC TIMESTAMP
    // ═════════════════════════════════════════════════════════════════
    self.last_sync = Utc::now();
    Ok(())
}
```

---

## Current Issue Detection Code (jira.rs:79-93)

```rust
pub async fn find_issue_from_activity(&self, activity: &Activity) -> Result<Option<String>> {
    // ────────────────────────────────────────────────────────────────────
    // CURRENT IMPLEMENTATION: SIMPLE REGEX
    // ────────────────────────────────────────────────────────────────────
    
    // Combine title and app name
    let text = format!("{} {}", activity.window_title, activity.app_name);
    
    // ⚠️ NOTICE: activity.description is IGNORED!
    // ⚠️ This field contains valuable screen capture text
    
    // Simple regex pattern
    let issue_key_regex = regex::Regex::new(r"([A-Z]+-\d+)").unwrap();
    
    // Try to find match
    if let Some(captures) = issue_key_regex.captures(&text) {
        if let Some(issue_key) = captures.get(1) {
            return Ok(Some(issue_key.as_str().to_string()));
        }
    }
    
    // ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
    // ┃  ENHANCED VERSION with LLM fallback:                           ┃
    // ┃                                                                ┃
    // ┃  pub async fn find_issue_from_activity(&self, activity: &Activity)
    // ┃      -> Result<Option<String>> {                              ┃
    // ┃      let text = format!("{} {}", activity.window_title,       ┃
    // ┃                                   activity.app_name);         ┃
    // ┃                                                                ┃
    // ┃      // Try regex first (fast path)                           ┃
    // ┃      let regex = regex::Regex::new(r"([A-Z]+-\d+)")?;        ┃
    // ┃      if let Some(cap) = regex.captures(&text) {              ┃
    // ┃          return Ok(cap.get(1).map(|m| m.as_str().to_string()));
    // ┃      }                                                         ┃
    // ┃                                                                ┃
    // ┃      // If regex fails, try LLM (with description!)          ┃
    // ┃      if let Some(llm) = &self.llm_client {                   ┃
    // ┃          if let Ok(Some(issue)) = llm.extract_issue_key(     ┃
    // ┃              activity.window_title,                          ┃
    // ┃              activity.app_name,                              ┃
    // ┃              activity.description,  // ◄── NOW USED!         ┃
    // ┃          ).await { return Ok(Some(issue)); }                 ┃
    // ┃      }                                                         ┃
    // ┃                                                                ┃
    // ┃      Ok(None)                                                 ┃
    // ┃  }                                                             ┃
    // ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
    
    Ok(None)
}
```

---

## Current Jira Logging Code (jira.rs:36-49)

```rust
pub async fn log_work(&self, issue_key: &str, activity: &Activity) -> Result<()> {
    let url = format!("{}/rest/api/3/issue/{}/worklog", self.base_url, issue_key);
    
    // ────────────────────────────────────────────────────────────────────
    // CURRENT COMMENT GENERATION: VERY SIMPLE
    // ────────────────────────────────────────────────────────────────────
    let worklog = WorklogEntry {
        comment: format!(
            "Auto-tracked: {} - {}",
            activity.app_name, 
            activity.window_title
        ),
        // Example output: "Auto-tracked: IntelliJ IDEA - PROJ-123: Feature X"
        //
        // ⚠️ PROBLEM: 
        // - Very generic
        // - Doesn't describe what was actually done
        // - activity.description is ignored
        // - No semantic understanding
        
        time_spent_seconds: activity.duration_secs,
        started: activity
            .timestamp
            .format("%Y-%m-%dT%H:%M:%S%.3f%z")
            .to_string(),
    };
    
    // ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
    // ┃  ENHANCED VERSION with LLM summarization:                   ┃
    // ┃                                                            ┃
    // ┃  let comment = if let Some(llm) = &self.llm_client {      ┃
    // ┃      match llm.summarize_activity(activity).await {      ┃
    // ┃          Ok(summary) => summary,                         ┃
    // ┃          Err(_) => format!("Auto-tracked: {} - {}",       ┃
    // ┃                             activity.app_name,           ┃
    // ┃                             activity.window_title)       ┃
    // ┃      }                                                     ┃
    // ┃  } else {                                                  ┃
    // ┃      format!("Auto-tracked: {} - {}",                    ┃
    // ┃              activity.app_name,                          ┃
    // ┃              activity.window_title)                      ┃
    // ┃  };                                                        ┃
    // ┃                                                            ┃
    // ┃  // Example output with LLM:                             ┃
    // ┃  // "Development: Reviewed PR #456, improved error       ┃
    // ┃  //  handling. Merged OAuth integration feature."        ┃
    // ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
    
    let response = self
        .client
        .post(&url)
        .basic_auth(&self.email, Some(&self.api_token))
        .json(&worklog)
        .send()
        .await
        .context("Failed to log work to Jira")?;
    
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("Jira API error ({}): {}", status, text);
    }
    
    let _result: JiraWorklogResponse = response
        .json()
        .await
        .context("Failed to parse Jira response")?;
    
    log::info!(
        "Logged {} seconds to Jira issue {}",
        activity.duration_secs,
        issue_key
    );
    Ok(())
}
```

---

## Consolidation Method (tracker.rs:167-182)

```rust
fn consolidate_activities(&self, activities: &[Activity]) -> Vec<Activity> {
    // ────────────────────────────────────────────────────────────────────
    // GROUP BY APP:WINDOW, SUM DURATIONS
    // ────────────────────────────────────────────────────────────────────
    let mut consolidated: HashMap<String, Activity> = HashMap::new();
    
    for activity in activities {
        // Create grouping key
        let key = format!("{}:{}", activity.app_name, activity.window_title);
        
        // Either create new entry or sum durations
        consolidated
            .entry(key)
            .and_modify(|existing| {
                existing.duration_secs += activity.duration_secs;
            })
            .or_insert_with(|| activity.clone());
    }
    
    consolidated.into_values().collect()
    
    // ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
    // ┃  AFTER THIS POINT: PERFECT PLACE FOR LLIM ENRICHMENT     ┃
    // ┃                                                          ┃
    // ┃  fn enrich_activities_with_llm(                         ┃
    // ┃      &self,                                             ┃
    // ┃      activities: &[Activity]                            ┃
    // ┃  ) -> Result<Vec<EnrichedActivity>> {                  ┃
    // ┃      for activity in activities {                       ┃
    // ┃          let analysis = self.llm                        ┃
    // ┃              .analyze_activity(activity)               ┃
    // ┃              .await?;                                  ┃
    // ┃          // Returns:                                    ┃
    // ┃          // - work_type (coding, meeting, etc)         ┃
    // ┃          // - key_topics                               ┃
    // ┃          // - suggested_projects                       ┃
    // ┃          // - summary                                  ┃
    // ┃      }                                                  ┃
    // ┃      Ok(enriched)                                       ┃
    // ┃  }                                                       ┃
    // ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
}
```

---

## Screenpipe Fetch Method (screenpipe.rs:51-111)

```rust
pub async fn get_recent_activities(&self, since: DateTime<Utc>) -> Result<Vec<Activity>> {
    let url = format!("{}/search", self.base_url);
    
    // ────────────────────────────────────────────────────────────────────
    // PREPARE REQUEST PARAMETERS
    // ────────────────────────────────────────────────────────────────────
    let params: HashMap<&str, String> = [
        ("start_timestamp", since.timestamp().to_string()),
        ("end_timestamp", Utc::now().timestamp().to_string()),
        ("limit", "100".to_string()),
    ]
    .iter()
    .cloned()
    .collect();
    
    // ────────────────────────────────────────────────────────────────────
    // MAKE HTTP REQUEST TO SCREENPIPE
    // ────────────────────────────────────────────────────────────────────
    let response = self
        .client
        .get(&url)
        .query(&params)
        .send()
        .await
        .context("Failed to fetch activities from Screenpipe")?;
    
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("Screenpipe API error ({}): {}", status, text);
    }
    
    let body = response
        .text()
        .await
        .context("Failed to read Screenpipe response body")?;
    
    debug!("Screenpipe response payload: {}", body);
    
    // ────────────────────────────────────────────────────────────────────
    // PARSE RESPONSE INTO RUST STRUCTS
    // ────────────────────────────────────────────────────────────────────
    let screenpipe_response: ScreenpipeResponse = serde_json::from_str(&body)
        .with_context(|| format!("Failed to parse Screenpipe response: {}", body))?;
    
    // ────────────────────────────────────────────────────────────────────
    // TRANSFORM TO ACTIVITY STRUCTS
    // ────────────────────────────────────────────────────────────────────
    let activities = screenpipe_response
        .data
        .into_iter()
        .filter_map(|entry| {
            // Parse timestamp
            let timestamp = entry
                .content
                .timestamp
                .as_deref()
                .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            
            // Create Activity struct
            Some(Activity {
                timestamp,
                duration_secs: 60,  // ⚠️ HARDCODED!
                window_title: entry.content.window_name.unwrap_or_default(),
                app_name: entry.content.app_name.unwrap_or_default(),
                description: entry.content.text.unwrap_or_default(),
                // ◄── This field is populated but RARELY USED!
            })
        })
        .collect();
    
    Ok(activities)
}
```

---

## Summary: Where to Add LLM Code

```
Screenpipe Data Input
         ↓
    FETCH (screenpipe.rs:51-111)
         ↓
  CONSOLIDATE (tracker.rs:167-182)
         ↓
  ┌───────────────────────────────────────────┐
  │  INJECTION POINT 1: Add here              │
  │  Call: enrich_with_llm(&activities)       │
  │  Classify work types, extract topics      │
  └───────────────────────────────────────────┘
         ↓
    FILTER (tracker.rs:112)
         ↓
  ┌───────────────────────────────────────────┐
  │  INJECTION POINT 2: Add here              │
  │  Call: jira.find_issue_with_llm(activity) │
  │  Fallback: regex only                     │
  └───────────────────────────────────────────┘
         ↓
  ┌───────────────────────────────────────────┐
  │  INJECTION POINT 3: Add here              │
  │  Call: llm.summarize(activity)            │
  │  Enhanced comment generation              │
  └───────────────────────────────────────────┘
         ↓
    LOG TO JIRA (jira.rs:36-77)
         ↓
   OUTPUT TO JIRA API
```

---

## Implementation Roadmap

### Phase 1: Smart Issue Detection (Start Here!)
**Files to modify:**
1. `jira.rs` - Enhance `find_issue_from_activity()`
2. `config.rs` - Add LLM configuration section
3. `tracker.rs` - Pass activity.description to detection
4. `llm.rs` (NEW) - Create LLM client module

**What to do:**
- Keep existing regex method
- Add LLM fallback if regex fails
- Use Activity.description field

### Phase 2: Comment Enrichment
**Files to modify:**
1. `jira.rs` - Enhance `log_work()`
2. `llm.rs` - Add `summarize_activity()` method

**What to do:**
- Replace generic comment template
- Use LLM to generate meaningful descriptions
- Keep character limit in mind (<280 chars for Jira)

### Phase 3: Activity Enrichment
**Files to modify:**
1. `tracker.rs` - Call enrichment after consolidation
2. `screenpipe.rs` - Extend Activity struct
3. `llm.rs` - Add `classify_activity()` method

**What to do:**
- Classify work types
- Extract key topics
- Tag technologies mentioned

### Phase 4: Batch Analysis
**Files to create/modify:**
1. NEW `reports.rs` - Batch analysis module
2. `daemon.rs` - Add reporting endpoint

**What to do:**
- Collect activities for period
- Generate summaries
- Identify patterns and trends

