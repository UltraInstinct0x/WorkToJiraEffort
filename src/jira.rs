use crate::llm::AssignedIssue;
use crate::screenpipe::Activity;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorklogEntry {
    pub comment: String,
    #[serde(rename = "timeSpentSeconds")]
    pub time_spent_seconds: u64,
    pub started: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct JiraWorklogResponse {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct JiraUser {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "emailAddress")]
    pub email_address: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct JiraIssue {
    pub key: String,
    pub fields: JiraIssueFields,
}

#[derive(Debug, Deserialize)]
pub struct JiraIssueFields {
    pub summary: String,
    pub assignee: Option<JiraAssignee>,
}

#[derive(Debug, Deserialize)]
pub struct JiraAssignee {
    #[serde(rename = "accountId")]
    pub account_id: String,
}

#[derive(Debug, Deserialize)]
pub struct JiraSearchResponse {
    pub issues: Vec<JiraIssue>,
    pub total: usize,
}

/// Cached assigned issues with timestamp
#[derive(Debug, Clone)]
struct AssignedIssuesCache {
    issues: Vec<AssignedIssue>,
    cached_at: DateTime<Utc>,
}

pub struct JiraClient {
    base_url: String,
    email: String,
    api_token: String,
    client: reqwest::Client,
    assigned_issues_cache: Arc<RwLock<Option<AssignedIssuesCache>>>,
    cache_duration_secs: u64,
}

impl JiraClient {
    pub fn new(base_url: String, email: String, api_token: String) -> Self {
        Self {
            base_url,
            email,
            api_token,
            client: reqwest::Client::new(),
            assigned_issues_cache: Arc::new(RwLock::new(None)),
            cache_duration_secs: 7200, // 2 hours default
        }
    }

    pub fn with_cache_duration(mut self, cache_duration_secs: u64) -> Self {
        self.cache_duration_secs = cache_duration_secs;
        self
    }

    pub async fn log_work(&self, issue_key: &str, activity: &Activity) -> Result<()> {
        let url = format!("{}/rest/api/3/issue/{}/worklog", self.base_url, issue_key);

        let worklog = WorklogEntry {
            comment: format!(
                "Auto-tracked: {} - {}",
                activity.app_name, activity.window_title
            ),
            time_spent_seconds: activity.duration_secs,
            started: activity
                .timestamp
                .format("%Y-%m-%dT%H:%M:%S%.3f%z")
                .to_string(),
        };

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

    pub async fn find_issue_from_activity(&self, activity: &Activity) -> Result<Option<String>> {
        // Simple heuristic: look for Jira issue keys (e.g., PROJ-123) in window title or app name
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

    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/rest/api/3/myself", self.base_url);

        match self
            .client
            .get(&url)
            .basic_auth(&self.email, Some(&self.api_token))
            .send()
            .await
        {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Get current user information
    pub async fn get_current_user(&self) -> Result<JiraUser> {
        let url = format!("{}/rest/api/3/myself", self.base_url);

        let response = self
            .client
            .get(&url)
            .basic_auth(&self.email, Some(&self.api_token))
            .send()
            .await
            .context("Failed to get current user from Jira")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Jira API error ({}): {}", status, text);
        }

        let user: JiraUser = response
            .json()
            .await
            .context("Failed to parse Jira user response")?;

        Ok(user)
    }

    /// Fetch issues assigned to the current user
    async fn fetch_assigned_issues_from_api(&self) -> Result<Vec<AssignedIssue>> {
        // Get current user first
        let user = self.get_current_user().await?;

        // JQL query to get issues assigned to current user
        let jql = format!("assignee = \"{}\" AND resolution = Unresolved ORDER BY updated DESC", user.account_id);

        let url = format!("{}/rest/api/3/search", self.base_url);

        log::debug!("Fetching assigned issues with JQL: {}", jql);

        let response = self
            .client
            .get(&url)
            .basic_auth(&self.email, Some(&self.api_token))
            .query(&[("jql", jql), ("maxResults", "100".to_string()), ("fields", "summary,assignee".to_string())])
            .send()
            .await
            .context("Failed to search for assigned issues")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Jira search API error ({}): {}", status, text);
        }

        let search_response: JiraSearchResponse = response
            .json()
            .await
            .context("Failed to parse Jira search response")?;

        let assigned_issues: Vec<AssignedIssue> = search_response
            .issues
            .into_iter()
            .map(|issue| AssignedIssue {
                key: issue.key,
                summary: issue.fields.summary,
            })
            .collect();

        log::info!("Fetched {} assigned issues from Jira", assigned_issues.len());

        Ok(assigned_issues)
    }

    /// Get assigned issues with caching
    pub async fn get_assigned_issues(&self) -> Result<Vec<AssignedIssue>> {
        // Check cache first
        {
            let cache = self.assigned_issues_cache.read().await;
            if let Some(cached) = cache.as_ref() {
                let age = Utc::now() - cached.cached_at;
                if age.num_seconds() < self.cache_duration_secs as i64 {
                    log::debug!(
                        "Using cached assigned issues ({} issues, cached {}s ago)",
                        cached.issues.len(),
                        age.num_seconds()
                    );
                    return Ok(cached.issues.clone());
                } else {
                    log::debug!("Assigned issues cache expired ({}s old)", age.num_seconds());
                }
            }
        }

        // Cache miss or expired, fetch from API
        let issues = self.fetch_assigned_issues_from_api().await?;

        // Update cache
        {
            let mut cache = self.assigned_issues_cache.write().await;
            *cache = Some(AssignedIssuesCache {
                issues: issues.clone(),
                cached_at: Utc::now(),
            });
        }

        Ok(issues)
    }

    /// Check if a specific issue is assigned to the current user
    pub async fn is_assigned_to_me(&self, issue_key: &str) -> Result<bool> {
        let assigned_issues = self.get_assigned_issues().await?;
        Ok(assigned_issues.iter().any(|i| i.key == issue_key))
    }

    /// Clear the assigned issues cache (useful for testing or manual refresh)
    pub async fn clear_cache(&self) {
        let mut cache = self.assigned_issues_cache.write().await;
        *cache = None;
        log::debug!("Cleared assigned issues cache");
    }
}
