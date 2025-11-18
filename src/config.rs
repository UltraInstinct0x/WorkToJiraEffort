use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub company: CompanyConfig,
    pub screenpipe: ScreenpipeConfig,
    pub jira: JiraConfig,
    pub salesforce: SalesforceConfig,
    pub tracking: TrackingConfig,
    pub llm: LLMConfig,
    pub nudging: NudgingConfig,
    pub analytics: AnalyticsConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScreenpipeConfig {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JiraConfig {
    pub url: String,
    pub email: String,
    pub api_token: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SalesforceConfig {
    pub instance_url: String,
    pub username: String,
    pub password: String,
    pub security_token: String,
    pub client_id: String,
    pub client_secret: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompanyConfig {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackingConfig {
    pub screenpipe_poll_interval_secs: u64,
    pub llm_batch_interval_secs: u64,
    pub min_activity_duration_secs: u64,
    pub micro_activity_threshold_secs: u64,
    pub analyze_on_stop: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LLMConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub api_key: String,
    pub timeout_secs: u64,
    pub confidence_threshold: f64,
    pub batch_size: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NudgingConfig {
    pub enabled: bool,
    pub cooldown_secs: u64,
    pub detect_assigned_issues_in_titles: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalyticsConfig {
    pub store_local: bool,
    pub database_path: String,
    pub retention_days: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            company: CompanyConfig {
                name: "Your Company Name".to_string(),
            },
            screenpipe: ScreenpipeConfig {
                url: "http://localhost:3030".to_string(),
            },
            jira: JiraConfig {
                url: "https://your-domain.atlassian.net".to_string(),
                email: "your-email@example.com".to_string(),
                api_token: "your-api-token".to_string(),
                enabled: true,
            },
            salesforce: SalesforceConfig {
                instance_url: "https://your-instance.salesforce.com".to_string(),
                username: "your-username".to_string(),
                password: "your-password".to_string(),
                security_token: "your-security-token".to_string(),
                client_id: "your-client-id".to_string(),
                client_secret: "your-client-secret".to_string(),
                enabled: false,
            },
            tracking: TrackingConfig {
                screenpipe_poll_interval_secs: 300, // 5 minutes
                llm_batch_interval_secs: 10800,      // 3 hours
                min_activity_duration_secs: 60,      // 1 minute
                micro_activity_threshold_secs: 600,  // 10 minutes
                analyze_on_stop: true,
            },
            llm: LLMConfig {
                enabled: false,
                endpoint: "https://your-corporate-api.company.com/ai/analyze".to_string(),
                api_key: "your-api-key".to_string(),
                timeout_secs: 30,
                confidence_threshold: 0.75,
                batch_size: 100,
            },
            nudging: NudgingConfig {
                enabled: true,
                cooldown_secs: 1800, // 30 minutes
                detect_assigned_issues_in_titles: true,
            },
            analytics: AnalyticsConfig {
                store_local: true,
                database_path: "~/.work-tracker/analytics.db".to_string(),
                retention_days: 90,
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            let config = Self::default();
            config.save()?;
            log::info!("Created default config at: {}", config_path.display());
            return Ok(config);
        }

        let content =
            std::fs::read_to_string(&config_path).context("Failed to read config file")?;

        let config: Config = toml::from_str(&content).context("Failed to parse config file")?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        std::fs::write(&config_path, content).context("Failed to write config file")?;

        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let config_dir =
            directories::ProjectDirs::from("com", "WorkToJiraEffort", "WorkToJiraEffort")
                .context("Failed to determine config directory")?
                .config_dir()
                .to_path_buf();

        Ok(config_dir.join("config.toml"))
    }
}
