use crate::screenpipe::Activity;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct SalesforceLoginRequest {
    pub grant_type: String,
    pub client_id: String,
    pub client_secret: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SalesforceLoginResponse {
    pub access_token: String,
    pub instance_url: String,
}

#[derive(Debug, Serialize)]
pub struct TimeEntry {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "StartTime__c")]
    pub start_time: String,
    #[serde(rename = "DurationMinutes__c")]
    pub duration_minutes: f64,
    #[serde(rename = "Description__c")]
    pub description: String,
}

pub struct SalesforceClient {
    instance_url: String,
    username: String,
    password: String,
    security_token: String,
    client_id: String,
    client_secret: String,
    client: reqwest::Client,
    access_token: Option<String>,
}

impl SalesforceClient {
    pub fn new(
        instance_url: String,
        username: String,
        password: String,
        security_token: String,
        client_id: String,
        client_secret: String,
    ) -> Self {
        Self {
            instance_url,
            username,
            password,
            security_token,
            client_id,
            client_secret,
            client: reqwest::Client::new(),
            access_token: None,
        }
    }

    async fn authenticate(&mut self) -> Result<()> {
        let url = format!("{}/services/oauth2/token", self.instance_url);

        let password_with_token = format!("{}{}", self.password, self.security_token);

        let params = [
            ("grant_type", "password"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("username", &self.username),
            ("password", &password_with_token),
        ];

        let response = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await
            .context("Failed to authenticate with Salesforce")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Salesforce authentication error ({}): {}", status, text);
        }

        let auth_response: SalesforceLoginResponse = response
            .json()
            .await
            .context("Failed to parse Salesforce auth response")?;

        self.access_token = Some(auth_response.access_token);
        log::info!("Successfully authenticated with Salesforce");
        Ok(())
    }

    pub async fn log_time(&mut self, activity: &Activity) -> Result<()> {
        // Ensure we have a valid token
        if self.access_token.is_none() {
            self.authenticate().await?;
        }

        let token = self
            .access_token
            .as_ref()
            .context("No access token available")?
            .clone();

        // Note: This uses a custom Time Entry object.
        // You may need to adjust this based on your Salesforce setup
        let url = format!(
            "{}/services/data/v58.0/sobjects/TimeEntry__c",
            self.instance_url
        );

        let time_entry = TimeEntry {
            name: format!("Auto-tracked: {}", activity.app_name),
            start_time: activity.timestamp.to_rfc3339(),
            duration_minutes: activity.duration_secs as f64 / 60.0,
            description: format!("{} - {}", activity.app_name, activity.window_title),
        };

        let response = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&time_entry)
            .send()
            .await
            .context("Failed to log time to Salesforce")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();

            // If unauthorized, try to re-authenticate once
            if status == reqwest::StatusCode::UNAUTHORIZED {
                log::warn!("Salesforce token expired, re-authenticating...");
                self.access_token = None;
                self.authenticate().await?;

                let new_token = self
                    .access_token
                    .as_ref()
                    .context("No access token after re-authentication")?;

                let retry_response = self
                    .client
                    .post(&url)
                    .bearer_auth(new_token)
                    .json(&time_entry)
                    .send()
                    .await
                    .context("Failed to log time to Salesforce after re-auth")?;

                if !retry_response.status().is_success() {
                    let retry_status = retry_response.status();
                    let retry_text = retry_response.text().await.unwrap_or_default();
                    anyhow::bail!(
                        "Salesforce API error after re-auth ({}): {}",
                        retry_status,
                        retry_text
                    );
                }

                log::info!(
                    "Logged {} minutes to Salesforce",
                    activity.duration_secs / 60
                );
                return Ok(());
            }

            anyhow::bail!("Salesforce API error ({}): {}", status, text);
        }

        log::info!(
            "Logged {} minutes to Salesforce",
            activity.duration_secs / 60
        );
        Ok(())
    }

    pub async fn health_check(&mut self) -> Result<bool> {
        if self.access_token.is_none() {
            match self.authenticate().await {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        } else {
            Ok(true)
        }
    }
}
