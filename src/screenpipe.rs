use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use log::debug;
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
    pub data: Vec<ScreenpipeSearchEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenpipeSearchEntry {
    #[serde(rename = "type")]
    pub data_type: String,
    pub content: ScreenpipeContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenpipeContent {
    pub frame_id: Option<i64>,
    pub text: Option<String>,
    pub timestamp: Option<String>,
    pub app_name: Option<String>,
    pub window_name: Option<String>,
    pub browser_url: Option<String>,
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

        let body = response
            .text()
            .await
            .context("Failed to read Screenpipe response body")?;

        debug!("Screenpipe response payload: {}", body);

        let screenpipe_response: ScreenpipeResponse = serde_json::from_str(&body)
            .with_context(|| format!("Failed to parse Screenpipe response: {}", body))?;

        let activities = screenpipe_response
            .data
            .into_iter()
            .filter_map(|entry| {
                let timestamp = entry
                    .content
                    .timestamp
                    .as_deref()
                    .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);

                Some(Activity {
                    timestamp,
                    duration_secs: 60,
                    window_title: entry.content.window_name.unwrap_or_default(),
                    app_name: entry.content.app_name.unwrap_or_default(),
                    description: entry.content.text.unwrap_or_default(),
                })
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
