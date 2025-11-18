use crate::{
    config::Config,
    database::{ActivityTier, Database},
    jira::JiraClient,
    llm::LLMAnalyzer,
    salesforce::SalesforceClient,
    screenpipe::{Activity, ScreenpipeClient},
    state::{StateManager, TrackingState},
};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

pub struct WorkTracker {
    config: Config,
    screenpipe: ScreenpipeClient,
    jira: Option<JiraClient>,
    salesforce: Option<SalesforceClient>,
    llm_analyzer: Option<LLMAnalyzer>,
    database: Database,
    pub state_manager: Arc<RwLock<StateManager>>,
    last_sync: DateTime<Utc>,
    last_llm_analysis: DateTime<Utc>,
    issue_override: Arc<RwLock<Option<String>>>,
}

impl WorkTracker {
    pub fn new(config: Config, issue_override: Arc<RwLock<Option<String>>>) -> Result<Self> {
        let screenpipe = ScreenpipeClient::new(config.screenpipe.url.clone());

        let jira = if config.jira.enabled {
            Some(JiraClient::new(
                config.jira.url.clone(),
                config.jira.email.clone(),
                config.jira.api_token.clone(),
            ))
        } else {
            None
        };

        let salesforce = if config.salesforce.enabled {
            Some(SalesforceClient::new(
                config.salesforce.instance_url.clone(),
                config.salesforce.username.clone(),
                config.salesforce.password.clone(),
                config.salesforce.security_token.clone(),
                config.salesforce.client_id.clone(),
                config.salesforce.client_secret.clone(),
            ))
        } else {
            None
        };

        let llm_analyzer = if config.llm.enabled {
            Some(LLMAnalyzer::new(
                config.llm.endpoint.clone(),
                config.llm.api_key.clone(),
                config.llm.timeout_secs,
            )?)
        } else {
            None
        };

        // Initialize database
        let db_path = Self::get_database_path(&config)?;
        let database = Database::new(db_path)?;

        let state_manager = Arc::new(RwLock::new(StateManager::new()));

        Ok(Self {
            config,
            screenpipe,
            jira,
            salesforce,
            llm_analyzer,
            database,
            state_manager,
            last_sync: Utc::now() - Duration::minutes(5),
            last_llm_analysis: Utc::now(),
            issue_override,
        })
    }

    fn get_database_path(config: &Config) -> Result<PathBuf> {
        let path_str = &config.analytics.database_path;

        // Expand ~ to home directory
        let expanded = if path_str.starts_with('~') {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .context("Could not determine home directory")?;
            path_str.replacen('~', &home, 1)
        } else {
            path_str.clone()
        };

        Ok(PathBuf::from(expanded))
    }

    pub async fn check_health(&mut self) -> Result<()> {
        log::info!("Checking service health...");

        let screenpipe_healthy = self.screenpipe.health_check().await?;
        log::info!("Screenpipe: {}", if screenpipe_healthy { "✓" } else { "✗" });

        if let Some(jira) = &self.jira {
            let jira_healthy = jira.health_check().await?;
            log::info!("Jira: {}", if jira_healthy { "✓" } else { "✗" });
        }

        if let Some(salesforce) = &mut self.salesforce {
            let sf_healthy = salesforce.health_check().await?;
            log::info!("Salesforce: {}", if sf_healthy { "✓" } else { "✗" });
        }

        Ok(())
    }

    /// Start tracking - creates new session
    pub async fn start_tracking(&mut self) -> Result<()> {
        let session_id = self.database.create_session()?;

        let mut state = self.state_manager.write().await;
        state.start_tracking(session_id)
            .map_err(|e| anyhow::anyhow!(e))?;

        log::info!("Started tracking session {}", session_id);
        Ok(())
    }

    /// Pause tracking - creates break period
    pub async fn pause_tracking(&mut self) -> Result<()> {
        let state = self.state_manager.read().await;
        let session_id = state.current_session()
            .ok_or_else(|| anyhow::anyhow!("No active session"))?
            .id;
        drop(state);

        let break_id = self.database.create_break(session_id)?;

        let mut state = self.state_manager.write().await;
        state.pause_tracking(break_id)
            .map_err(|e| anyhow::anyhow!(e))?;

        log::info!("Paused tracking (break started)");
        Ok(())
    }

    /// Resume tracking from pause
    pub async fn resume_tracking(&mut self) -> Result<()> {
        let state = self.state_manager.read().await;
        let break_id = state.current_break()
            .ok_or_else(|| anyhow::anyhow!("No active break"))?
            .id;
        drop(state);

        self.database.end_break(break_id)?;

        let mut state = self.state_manager.write().await;
        state.resume_tracking()
            .map_err(|e| anyhow::anyhow!(e))?;

        log::info!("Resumed tracking");
        Ok(())
    }

    /// Stop tracking - ends session and triggers analysis
    pub async fn stop_tracking(&mut self) -> Result<()> {
        let state = self.state_manager.read().await;
        let session_id = state.current_session()
            .ok_or_else(|| anyhow::anyhow!("No active session"))?
            .id;
        drop(state);

        self.database.end_session(session_id)?;

        let mut state = self.state_manager.write().await;
        state.stop_tracking()
            .map_err(|e| anyhow::anyhow!(e))?;

        log::info!("Stopped tracking session {}", session_id);

        // Trigger final analysis if configured
        if self.config.tracking.analyze_on_stop {
            drop(state);
            self.analyze_and_log_batch(session_id).await?;
        }

        Ok(())
    }

    /// Sync activities from screenpipe to local database
    /// This runs every 5 minutes when tracking is active
    pub async fn sync(&mut self) -> Result<()> {
        let state = self.state_manager.read().await;
        let current_state = state.current_state();

        // Only collect activities when actively tracking
        if !current_state.is_tracking() {
            log::debug!("Not tracking, skipping sync");
            return Ok(());
        }

        let session_id = state.current_session()
            .ok_or_else(|| anyhow::anyhow!("No active session during tracking"))?
            .id;
        drop(state);

        log::info!("Fetching activities since {}", self.last_sync);

        let activities = self
            .screenpipe
            .get_recent_activities(self.last_sync)
            .await?;
        log::info!("Found {} activities", activities.len());

        if activities.is_empty() {
            self.last_sync = Utc::now();
            return Ok(());
        }

        // Consolidate and store activities
        let consolidated = self.consolidate_activities(&activities);
        log::info!("Consolidated into {} entries", consolidated.len());

        for activity in &consolidated {
            self.database.store_activity(session_id, activity)?;
            log::debug!(
                "Stored: {} - {} ({}s, tier: {:?})",
                activity.app_name,
                activity.window_title,
                activity.duration_secs,
                ActivityTier::from_duration(activity.duration_secs)
            );
        }

        self.last_sync = Utc::now();
        Ok(())
    }

    /// Analyze buffered activities using LLM and log to Jira
    /// This runs every 3 hours or when tracking stops
    pub async fn analyze_and_log_batch(&mut self, session_id: i64) -> Result<()> {
        log::info!("Starting LLM batch analysis for session {}", session_id);

        // Get session statistics
        let stats = self.database.get_session_stats(session_id)?;
        log::info!(
            "Session stats: {} total activities ({} billable, {} micro)",
            stats.total_activities,
            stats.billable_activities,
            stats.micro_activities
        );

        // Get activities by tier
        let billable = self.database.get_session_activities(session_id, Some(ActivityTier::Billable))?;
        let micro = self.database.get_session_activities(session_id, Some(ActivityTier::Micro))?;

        if billable.is_empty() && micro.is_empty() {
            log::info!("No activities to analyze");
            return Ok(());
        }

        // If LLM is enabled, use it for analysis
        if let (Some(llm), Some(jira)) = (&self.llm_analyzer, &self.jira) {
            log::info!("Using LLM for batch analysis");

            // Get assigned issues
            let assigned_issues = jira.get_assigned_issues().await?;
            log::info!("Fetched {} assigned issues", assigned_issues.len());

            if assigned_issues.is_empty() {
                log::warn!("No assigned issues found - cannot match activities");
                return Ok(());
            }

            // Prepare LLM request
            let analysis_result = llm.analyze_batch(
                self.config.jira.email.clone(),
                self.config.company.name.clone(),
                assigned_issues,
                stats.start_time,
                stats.end_time.unwrap_or_else(Utc::now),
                stats.total_duration_secs,
                stats.break_duration_secs,
                billable,
                micro,
            ).await?;

            log::info!(
                "LLM analysis complete: {} issues matched, confidence: {:.2}",
                analysis_result.analysis.issues.len(),
                analysis_result.analysis.confidence
            );

            // Store analysis result
            let analysis_json = serde_json::to_string(&analysis_result)?;
            self.database.store_analysis(
                session_id,
                analysis_json,
                analysis_result.analysis.confidence,
            )?;

            // Log to Jira based on LLM results
            for issue_match in &analysis_result.analysis.issues {
                if issue_match.confidence < self.config.llm.confidence_threshold {
                    log::warn!(
                        "Skipping {} - confidence too low: {:.2}",
                        issue_match.key,
                        issue_match.confidence
                    );
                    continue;
                }

                // Create worklog entry with LLM-generated summary
                let activity = Activity {
                    timestamp: stats.start_time,
                    duration_secs: issue_match.total_time_secs,
                    window_title: issue_match.summary.clone(),
                    app_name: self.config.company.name.clone(),
                    description: format!("Work type: {}", issue_match.work_type),
                };

                match jira.log_work(&issue_match.key, &activity).await {
                    Ok(_) => {
                        log::info!(
                            "Logged {} to {} ({} mins)",
                            issue_match.key,
                            issue_match.summary,
                            issue_match.total_time_secs / 60
                        );

                        // Mark activities as logged
                        self.database.mark_activities_logged(&issue_match.activities_included)?;
                    }
                    Err(e) => {
                        log::error!("Failed to log to Jira {}: {}", issue_match.key, e);
                    }
                }
            }

            // Report unmatched activities
            if analysis_result.analysis.unmatched.total_time_secs > 0 {
                log::warn!(
                    "Unmatched time: {} mins ({})",
                    analysis_result.analysis.unmatched.total_time_secs / 60,
                    analysis_result.analysis.unmatched.likely_reason
                );
            }

        } else {
            log::info!("LLM disabled, using fallback regex matching");
            // Fallback to regex-based matching (original behavior)
            self.fallback_regex_logging(session_id, &billable).await?;
        }

        self.last_llm_analysis = Utc::now();
        Ok(())
    }

    /// Fallback regex-based logging (original behavior)
    async fn fallback_regex_logging(&mut self, session_id: i64, activities: &[crate::database::StoredActivity]) -> Result<()> {
        if let Some(jira) = &self.jira {
            let issue_override = {
                let guard = self.issue_override.read().await;
                guard.clone()
            };

            for stored_activity in activities {
                if stored_activity.logged_to_jira {
                    continue;
                }

                let activity = Activity {
                    timestamp: stored_activity.timestamp,
                    duration_secs: stored_activity.duration_secs,
                    window_title: stored_activity.window_title.clone(),
                    app_name: stored_activity.app_name.clone(),
                    description: stored_activity.description.clone(),
                };

                let target_issue = if let Some(issue_key) = &issue_override {
                    Some(issue_key.clone())
                } else {
                    match jira.find_issue_from_activity(&activity).await {
                        Ok(result) => result,
                        Err(err) => {
                            log::error!("Failed to detect Jira issue: {}", err);
                            None
                        }
                    }
                };

                if let Some(issue_key) = target_issue {
                    // Check if assigned to user
                    match jira.is_assigned_to_me(&issue_key).await {
                        Ok(true) => {
                            match jira.log_work(&issue_key, &activity).await {
                                Ok(_) => {
                                    log::info!("Logged to Jira: {}", issue_key);
                                    self.database.mark_activities_logged(&[stored_activity.id])?;
                                }
                                Err(e) => log::error!("Failed to log to Jira: {}", e),
                            }
                        }
                        Ok(false) => {
                            log::warn!("Skipping {} - not assigned to you", issue_key);
                        }
                        Err(e) => {
                            log::error!("Failed to check assignment for {}: {}", issue_key, e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn consolidate_activities(&self, activities: &[Activity]) -> Vec<Activity> {
        let mut consolidated: HashMap<String, Activity> = HashMap::new();

        for activity in activities {
            let key = format!("{}:{}", activity.app_name, activity.window_title);

            consolidated
                .entry(key)
                .and_modify(|existing| {
                    existing.duration_secs += activity.duration_secs;
                })
                .or_insert_with(|| activity.clone());
        }

        consolidated.into_values().collect()
    }

    /// Main run loop with state-aware polling
    pub async fn run(&mut self, interval_secs: u64) -> Result<()> {
        log::info!(
            "Starting work tracker (polling every {} seconds)...",
            interval_secs
        );

        let llm_interval_secs = self.config.tracking.llm_batch_interval_secs;

        loop {
            // Screenpipe sync (every 5 min)
            match self.sync().await {
                Ok(_) => log::debug!("Sync completed successfully"),
                Err(e) => log::error!("Sync failed: {:#}", e),
            }

            // Check if it's time for LLM analysis (every 3 hours)
            let since_last_analysis = Utc::now() - self.last_llm_analysis;
            if since_last_analysis.num_seconds() >= llm_interval_secs as i64 {
                let state = self.state_manager.read().await;
                if let Some(session) = state.current_session() {
                    let session_id = session.id;
                    drop(state);

                    log::info!("Triggering scheduled LLM analysis");
                    match self.analyze_and_log_batch(session_id).await {
                        Ok(_) => log::info!("Scheduled analysis completed"),
                        Err(e) => log::error!("Scheduled analysis failed: {:#}", e),
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
        }
    }
}
