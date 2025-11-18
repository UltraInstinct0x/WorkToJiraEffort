use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::database::StoredActivity;

/// Jira issue information for the LLM context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignedIssue {
    pub key: String,
    pub summary: String,
}

/// Activity data sent to LLM for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityForAnalysis {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub duration_secs: u64,
    pub app_name: String,
    pub window_title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ocr_sample: String, // Limited sample of description to avoid sending too much data
}

impl From<&StoredActivity> for ActivityForAnalysis {
    fn from(activity: &StoredActivity) -> Self {
        // Limit OCR text to first 500 chars to avoid overwhelming the LLM
        let ocr_sample = if activity.description.len() > 500 {
            format!("{}...", &activity.description[..500])
        } else {
            activity.description.clone()
        };

        Self {
            id: activity.id,
            timestamp: activity.timestamp,
            duration_secs: activity.duration_secs,
            app_name: activity.app_name.clone(),
            window_title: activity.window_title.clone(),
            ocr_sample,
        }
    }
}

/// Request payload sent to corporate LLM API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMAnalysisRequest {
    pub user: UserContext,
    pub session: SessionContext,
    pub activities: ActivitiesContext,
    pub task: TaskInstructions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub email: String,
    pub company: String,
    pub assigned_issues: Vec<AssignedIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub tracking_duration_secs: u64,
    pub break_duration_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitiesContext {
    pub billable: Vec<ActivityForAnalysis>,
    pub micro: Vec<ActivityForAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInstructions {
    pub primary: String,
    pub rules: Vec<String>,
}

/// LLM analysis response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMAnalysisResponse {
    pub analysis: AnalysisResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub total_productive_time_secs: u64,
    pub confidence: f64,
    pub issues: Vec<IssueMatch>,
    pub unmatched: UnmatchedActivities,
    pub micro_activities_merged: bool,
    pub red_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueMatch {
    pub key: String,
    pub total_time_secs: u64,
    pub summary: String,
    pub work_type: String,
    pub activities_included: Vec<i64>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmatchedActivities {
    pub total_time_secs: u64,
    pub activities: Vec<i64>,
    pub likely_reason: String,
}

/// LLM analyzer client for corporate API
pub struct LLMAnalyzer {
    endpoint: String,
    api_key: String,
    timeout: Duration,
    client: reqwest::Client,
}

impl LLMAnalyzer {
    pub fn new(endpoint: String, api_key: String, timeout_secs: u64) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            endpoint,
            api_key,
            timeout: Duration::from_secs(timeout_secs),
            client,
        })
    }

    /// Analyze a batch of activities using the corporate LLM API
    pub async fn analyze_batch(
        &self,
        user_email: String,
        company_name: String,
        assigned_issues: Vec<AssignedIssue>,
        session_start: DateTime<Utc>,
        session_end: DateTime<Utc>,
        tracking_duration_secs: u64,
        break_duration_secs: u64,
        billable_activities: Vec<StoredActivity>,
        micro_activities: Vec<StoredActivity>,
    ) -> Result<LLMAnalysisResponse> {
        let request = LLMAnalysisRequest {
            user: UserContext {
                email: user_email,
                company: company_name,
                assigned_issues,
            },
            session: SessionContext {
                start: session_start,
                end: session_end,
                tracking_duration_secs,
                break_duration_secs,
            },
            activities: ActivitiesContext {
                billable: billable_activities
                    .iter()
                    .map(ActivityForAnalysis::from)
                    .collect(),
                micro: micro_activities
                    .iter()
                    .map(ActivityForAnalysis::from)
                    .collect(),
            },
            task: TaskInstructions {
                primary: "Analyze this work session. Group activities by issue, generate summaries, calculate productive time. ONLY match to assigned issues. Return grouped results.".to_string(),
                rules: vec![
                    "ONLY match to assigned_issues list".to_string(),
                    "Combine micro-activities with related billable activities when logical".to_string(),
                    "Generate summaries max 200 characters".to_string(),
                    "Return confidence scores (0-1)".to_string(),
                    "Flag unmatched activities (possible personal/other client work)".to_string(),
                    "Calculate actual productive time per issue".to_string(),
                ],
            },
        };

        log::debug!(
            "Sending LLM analysis request for {} billable and {} micro activities",
            billable_activities.len(),
            micro_activities.len()
        );

        let response = self
            .client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to LLM API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());
            anyhow::bail!("LLM API request failed with status {}: {}", status, body);
        }

        let llm_response: LLMAnalysisResponse = response
            .json()
            .await
            .context("Failed to parse LLM API response")?;

        log::info!(
            "LLM analysis completed: {} issues matched, confidence: {:.2}",
            llm_response.analysis.issues.len(),
            llm_response.analysis.confidence
        );

        Ok(llm_response)
    }

    /// Simple issue detection using LLM for a single activity
    /// This is used as a fallback when regex detection fails
    pub async fn suggest_issue(
        &self,
        activity: &StoredActivity,
        assigned_issues: &[AssignedIssue],
    ) -> Result<Option<String>> {
        // Create a minimal request for single activity analysis
        let activity_for_analysis = ActivityForAnalysis::from(activity);

        let request = serde_json::json!({
            "user": {
                "assigned_issues": assigned_issues,
            },
            "activity": activity_for_analysis,
            "task": "Identify which assigned issue this activity relates to. Return only the issue key or null if uncertain. Max 10 words."
        });

        log::debug!("Requesting LLM issue suggestion for activity: {}", activity.id);

        let response = self
            .client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send issue suggestion request to LLM API")?;

        if !response.status().is_success() {
            let status = response.status();
            log::warn!("LLM issue suggestion failed with status: {}", status);
            return Ok(None);
        }

        // Parse response - expecting simple JSON with "issue_key" field
        let response_json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse LLM issue suggestion response")?;

        if let Some(issue_key) = response_json.get("issue_key").and_then(|v| v.as_str()) {
            // Verify the suggested issue is in the assigned list
            if assigned_issues.iter().any(|i| i.key == issue_key) {
                log::info!("LLM suggested issue: {}", issue_key);
                return Ok(Some(issue_key.to_string()));
            } else {
                log::warn!("LLM suggested non-assigned issue: {}", issue_key);
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_serialization() {
        let activity = StoredActivity {
            id: 1,
            session_id: 1,
            timestamp: Utc::now(),
            duration_secs: 300,
            window_title: "Test".to_string(),
            app_name: "Test App".to_string(),
            description: "Test description".to_string(),
            tier: crate::database::ActivityTier::Micro,
            logged_to_jira: false,
        };

        let for_analysis = ActivityForAnalysis::from(&activity);
        assert_eq!(for_analysis.id, 1);
        assert_eq!(for_analysis.duration_secs, 300);
    }

    #[test]
    fn test_ocr_truncation() {
        let long_text = "a".repeat(1000);
        let activity = StoredActivity {
            id: 1,
            session_id: 1,
            timestamp: Utc::now(),
            duration_secs: 300,
            window_title: "Test".to_string(),
            app_name: "Test App".to_string(),
            description: long_text,
            tier: crate::database::ActivityTier::Micro,
            logged_to_jira: false,
        };

        let for_analysis = ActivityForAnalysis::from(&activity);
        assert!(for_analysis.ocr_sample.len() <= 503); // 500 + "..."
    }
}
