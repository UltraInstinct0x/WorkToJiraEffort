use crate::{config::Config, database::Database, screenpipe_manager::ScreenpipeManager, tracker::WorkTracker};
use anyhow::{Context, Result};
use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{net::TcpListener, signal, sync::{Mutex, RwLock}};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run the long-lived daemon that can be controlled by external clients (e.g., menubar app)
pub async fn run_daemon(port: u16, mut screenpipe: ScreenpipeManager) -> Result<()> {
    // On macOS, if launched from tray app, don't show in dock
    #[cfg(target_os = "macos")]
    if std::env::var("WORK_TO_JIRA_NO_DOCK").is_ok() {
        unsafe {
            use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicy};
            use cocoa::base::nil;

            let app = NSApp();
            if app != nil {
                app.setActivationPolicy_(
                    NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory,
                );
            }
        }
    }

    let config = Config::load().context("Failed to load configuration")?;
    let issue_override = Arc::new(RwLock::new(None));

    // Initialize database
    let db_path = get_database_path(&config)?;
    let database = Arc::new(Mutex::new(Database::new(db_path)?));

    // Start tracker loop in the background
    {
        let tracker_issue_override = Arc::clone(&issue_override);
        let config_clone = config.clone();

        tokio::spawn(async move {
            let interval = config_clone.tracking.screenpipe_poll_interval_secs;

            match WorkTracker::new(config_clone, tracker_issue_override) {
                Ok(mut tracker) => {
                    if let Err(err) = tracker.run(interval).await {
                        log::error!("Tracker daemon exited with error: {}", err);
                    }
                }
                Err(err) => {
                    log::error!("Failed to create tracker: {}", err);
                }
            }
        });
    }

    let state = Arc::new(DaemonState {
        issue_override,
        database,
    });

    let app = Router::new()
        .route("/status", get(status_handler))
        .route("/issue", post(issue_override_handler))
        .route("/export", get(export_handler))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    log::info!("WorkToJiraEffort daemon listening on http://{}", addr);
    let listener = TcpListener::bind(addr)
        .await
        .context("Failed to bind daemon TCP listener")?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Daemon HTTP server error")?;

    // Stop Screenpipe server when daemon shuts down
    log::info!("Daemon shutting down, stopping Screenpipe...");
    screenpipe.stop().await?;

    Ok(())
}

#[derive(Clone)]
struct DaemonState {
    issue_override: Arc<RwLock<Option<String>>>,
    database: Arc<Mutex<Database>>,
}

#[derive(Serialize)]
struct StatusResponse {
    version: &'static str,
    issue_override: Option<String>,
}

async fn status_handler(State(state): State<Arc<DaemonState>>) -> Json<StatusResponse> {
    let issue_override = state.issue_override.read().await.clone();
    Json(StatusResponse {
        version: VERSION,
        issue_override,
    })
}

#[derive(Deserialize)]
struct IssueRequest {
    issue_key: Option<String>,
}

async fn issue_override_handler(
    State(state): State<Arc<DaemonState>>,
    Json(payload): Json<IssueRequest>,
) -> Json<StatusResponse> {
    let cleaned = payload.issue_key.and_then(|value| {
        let trimmed = value.trim().to_uppercase();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

    {
        let mut guard = state.issue_override.write().await;
        *guard = cleaned;
    }

    status_handler(State(state)).await
}

async fn shutdown_signal() {
    if let Err(err) = signal::ctrl_c().await {
        log::warn!("Failed to listen for shutdown signal: {}", err);
    }
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

#[derive(Deserialize)]
struct ExportParams {
    #[serde(default = "default_format")]
    format: String,
}

fn default_format() -> String {
    "csv".to_string()
}

async fn export_handler(
    State(state): State<Arc<DaemonState>>,
    Query(params): Query<ExportParams>,
) -> Result<axum::response::Response, String> {
    let db = state.database.lock().await;

    // Get active session
    let session = db
        .get_active_session()
        .map_err(|e| format!("Failed to get session: {}", e))?;

    if session.is_none() {
        return Err("No active session found".to_string());
    }

    let session = session.unwrap();

    // Get all activities for the session
    let activities = db
        .get_session_activities(session.id, None)
        .map_err(|e| format!("Failed to get activities: {}", e))?;

    match params.format.as_str() {
        "csv" => {
            let mut csv = String::from("Timestamp,Duration (seconds),Window Title,App Name,Description,Tier,Logged to Jira\n");
            for activity in activities {
                csv.push_str(&format!(
                    "\"{}\",{},\"{}\",\"{}\",\"{}\",{},{}\n",
                    activity.timestamp.to_rfc3339(),
                    activity.duration_secs,
                    activity.window_title.replace('"', "\"\""),
                    activity.app_name.replace('"', "\"\""),
                    activity.description.replace('"', "\"\""),
                    activity.tier.as_str(),
                    if activity.logged_to_jira { "Yes" } else { "No" }
                ));
            }
            Ok(axum::response::Response::builder()
                .header("Content-Type", "text/csv")
                .body(csv.into())
                .unwrap())
        }
        "json" => {
            let json_activities: Vec<_> = activities
                .iter()
                .map(|a| {
                    serde_json::json!({
                        "timestamp": a.timestamp.to_rfc3339(),
                        "duration_secs": a.duration_secs,
                        "window_title": a.window_title,
                        "app_name": a.app_name,
                        "description": a.description,
                        "tier": a.tier.as_str(),
                        "logged_to_jira": a.logged_to_jira,
                    })
                })
                .collect();

            let json = serde_json::to_string_pretty(&json_activities)
                .map_err(|e| format!("Failed to serialize to JSON: {}", e))?;

            Ok(axum::response::Response::builder()
                .header("Content-Type", "application/json")
                .body(json.into())
                .unwrap())
        }
        _ => Err(format!("Unsupported format: {}", params.format)),
    }
}
