use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::screenpipe::Activity;

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

pub struct JiraClient {
    base_url: String,
    email: String,
    api_token: String,
    client: reqwest::Client,
}

impl JiraClient {
    pub fn new(base_url: String, email: String, api_token: String) -> Self {
        Self {
            base_url,
            email,
            api_token,
            client: reqwest::Client::new(),
        }
    }

    pub async fn log_work(&self, issue_key: &str, activity: &Activity) -> Result<()> {
        let url = format!("{}/rest/api/3/issue/{}/worklog", self.base_url, issue_key);

        let worklog = WorklogEntry {
            comment: format!(
                "Auto-tracked: {} - {}",
                activity.app_name,
                activity.window_title
            ),
            time_spent_seconds: activity.duration_secs,
            started: activity.timestamp.to_rfc3339(),
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

        log::info!("Logged {} seconds to Jira issue {}", activity.duration_secs, issue_key);
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
}
