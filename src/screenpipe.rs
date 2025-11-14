use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub timestamp: DateTime<Utc>,
    pub duration_secs: u64,
    pub window_title: String,
    pub app_name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenpipeResponse {
    pub data: Vec<ScreenpipeActivity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenpipeActivity {
    pub timestamp: i64,
    pub window_name: Option<String>,
    pub app_name: Option<String>,
    pub text_content: Option<String>,
}

pub struct ScreenpipeClient {
    base_url: String,
    client: reqwest::Client,
}

impl ScreenpipeClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_recent_activities(&self, since: DateTime<Utc>) -> Result<Vec<Activity>> {
        let url = format!("{}/search", self.base_url);

        // Screenpipe API parameters
        let params: HashMap<&str, String> = [
            ("start_timestamp", since.timestamp().to_string()),
            ("end_timestamp", Utc::now().timestamp().to_string()),
            ("limit", "100".to_string()),
        ]
        .iter()
        .cloned()
        .collect();

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

        let screenpipe_response: ScreenpipeResponse = response
            .json()
            .await
            .context("Failed to parse Screenpipe response")?;

        let activities = screenpipe_response
            .data
            .into_iter()
            .map(|sp_activity| Activity {
                timestamp: DateTime::from_timestamp(sp_activity.timestamp, 0)
                    .unwrap_or_else(Utc::now),
                duration_secs: 60, // Default duration, could be calculated
                window_title: sp_activity.window_name.unwrap_or_default(),
                app_name: sp_activity.app_name.unwrap_or_default(),
                description: sp_activity.text_content.unwrap_or_default(),
            })
            .collect();

        Ok(activities)
    }

    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}
