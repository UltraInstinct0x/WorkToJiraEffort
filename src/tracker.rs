use crate::{
    config::Config,
    jira::JiraClient,
    salesforce::SalesforceClient,
    screenpipe::{Activity, ScreenpipeClient},
};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

pub struct WorkTracker {
    config: Config,
    screenpipe: ScreenpipeClient,
    jira: Option<JiraClient>,
    salesforce: Option<SalesforceClient>,
    last_sync: DateTime<Utc>,
    issue_override: Arc<RwLock<Option<String>>>,
}

impl WorkTracker {
    pub fn new(config: Config, issue_override: Arc<RwLock<Option<String>>>) -> Self {
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

        Self {
            config,
            screenpipe,
            jira,
            salesforce,
            last_sync: Utc::now() - Duration::minutes(5),
            issue_override,
        }
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

    pub async fn sync(&mut self) -> Result<()> {
        log::info!("Fetching activities since {}", self.last_sync);

        let activities = self
            .screenpipe
            .get_recent_activities(self.last_sync)
            .await?;
        log::info!("Found {} activities", activities.len());

        if activities.is_empty() {
            return Ok(());
        }

        // Group activities by app/window to consolidate time
        let consolidated = self.consolidate_activities(&activities);
        log::info!("Consolidated into {} entries", consolidated.len());

        // Log to Jira
        if let Some(jira) = &self.jira {
            let issue_override = {
                let guard = self.issue_override.read().await;
                guard.clone()
            };

            for activity in &consolidated {
                if activity.duration_secs >= self.config.tracking.min_activity_duration_secs {
                    let target_issue = if let Some(issue_key) = &issue_override {
                        Some(issue_key.clone())
                    } else {
                        match jira.find_issue_from_activity(activity).await {
                            Ok(result) => result,
                            Err(err) => {
                                log::error!("Failed to detect Jira issue: {}", err);
                                None
                            }
                        }
                    };

                    if let Some(issue_key) = target_issue {
                        match jira.log_work(&issue_key, activity).await {
                            Ok(_) => log::info!("Successfully logged to Jira: {}", issue_key),
                            Err(e) => log::error!("Failed to log to Jira: {}", e),
                        }
                    }
                }
            }
        }

        // Log to Salesforce
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

        self.last_sync = Utc::now();
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

    pub async fn run(&mut self, interval_secs: u64) -> Result<()> {
        log::info!(
            "Starting work tracker (polling every {} seconds)...",
            interval_secs
        );

        loop {
            match self.sync().await {
                Ok(_) => log::debug!("Sync completed successfully"),
                Err(e) => log::error!("Sync failed: {:#}", e),
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
        }
    }
}
