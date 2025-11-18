use crate::{config::Config, state::TrackingState, tracker::WorkTracker};
use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, signal, sync::RwLock};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run the long-lived daemon that can be controlled by external clients (e.g., menubar app)
pub async fn run_daemon(port: u16) -> Result<()> {
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

    // Create tracker
    let tracker = WorkTracker::new(config.clone(), Arc::clone(&issue_override))?;
    let tracker = Arc::new(RwLock::new(tracker));

    // Start sync loop in the background
    {
        let tracker_clone = Arc::clone(&tracker);
        let interval = config.tracking.screenpipe_poll_interval_secs;

        tokio::spawn(async move {
            loop {
                {
                    let mut tracker = tracker_clone.write().await;
                    if let Err(err) = tracker.sync().await {
                        log::error!("Sync failed: {:#}", err);
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
            }
        });
    }

    // Start LLM analysis loop in the background
    {
        let tracker_clone = Arc::clone(&tracker);
        let llm_interval = config.tracking.llm_batch_interval_secs;

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(llm_interval)).await;

                let mut tracker = tracker_clone.write().await;

                // Get current session if tracking
                let session_id = {
                    let state_manager = tracker.state_manager.read().await;
                    state_manager.current_session().map(|s| s.id)
                };

                if let Some(session_id) = session_id {
                    log::info!("Running scheduled LLM analysis");
                    if let Err(err) = tracker.analyze_and_log_batch(session_id).await {
                        log::error!("Scheduled LLM analysis failed: {:#}", err);
                    }
                }
            }
        });
    }

    let state = Arc::new(DaemonState {
        tracker,
        issue_override,
    });

    let app = Router::new()
        .route("/status", get(status_handler))
        .route("/issue", post(issue_override_handler))
        .route("/start", post(start_tracking_handler))
        .route("/pause", post(pause_tracking_handler))
        .route("/resume", post(resume_tracking_handler))
        .route("/stop", post(stop_tracking_handler))
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

    Ok(())
}

#[derive(Clone)]
struct DaemonState {
    tracker: Arc<RwLock<WorkTracker>>,
    issue_override: Arc<RwLock<Option<String>>>,
}

#[derive(Serialize)]
struct StatusResponse {
    version: &'static str,
    tracking_state: String,
    session_duration_secs: Option<u64>,
    break_duration_secs: Option<u64>,
    issue_override: Option<String>,
}

async fn status_handler(State(state): State<Arc<DaemonState>>) -> Json<StatusResponse> {
    let tracker = state.tracker.read().await;
    let state_manager = tracker.state_manager.read().await;

    let tracking_state = state_manager.current_state();
    let session_duration = state_manager.current_session().map(|s| s.duration_secs());
    let break_duration = state_manager.current_break().map(|b| b.duration_secs());
    drop(state_manager);
    drop(tracker);

    let issue_override = state.issue_override.read().await.clone();

    Json(StatusResponse {
        version: VERSION,
        tracking_state: tracking_state.as_str().to_string(),
        session_duration_secs: session_duration,
        break_duration_secs: break_duration,
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

#[derive(Serialize)]
struct ActionResponse {
    success: bool,
    message: String,
    #[serde(flatten)]
    status: StatusResponse,
}

async fn start_tracking_handler(
    State(state): State<Arc<DaemonState>>,
) -> Result<Json<ActionResponse>, (StatusCode, String)> {
    let mut tracker = state.tracker.write().await;

    match tracker.start_tracking().await {
        Ok(_) => {
            drop(tracker);
            let status = status_handler(State(state.clone())).await.0;
            Ok(Json(ActionResponse {
                success: true,
                message: "Tracking started".to_string(),
                status,
            }))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            format!("Failed to start tracking: {}", e),
        )),
    }
}

async fn pause_tracking_handler(
    State(state): State<Arc<DaemonState>>,
) -> Result<Json<ActionResponse>, (StatusCode, String)> {
    let mut tracker = state.tracker.write().await;

    match tracker.pause_tracking().await {
        Ok(_) => {
            drop(tracker);
            let status = status_handler(State(state.clone())).await.0;
            Ok(Json(ActionResponse {
                success: true,
                message: "Tracking paused".to_string(),
                status,
            }))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            format!("Failed to pause tracking: {}", e),
        )),
    }
}

async fn resume_tracking_handler(
    State(state): State<Arc<DaemonState>>,
) -> Result<Json<ActionResponse>, (StatusCode, String)> {
    let mut tracker = state.tracker.write().await;

    match tracker.resume_tracking().await {
        Ok(_) => {
            drop(tracker);
            let status = status_handler(State(state.clone())).await.0;
            Ok(Json(ActionResponse {
                success: true,
                message: "Tracking resumed".to_string(),
                status,
            }))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            format!("Failed to resume tracking: {}", e),
        )),
    }
}

async fn stop_tracking_handler(
    State(state): State<Arc<DaemonState>>,
) -> Result<Json<ActionResponse>, (StatusCode, String)> {
    let mut tracker = state.tracker.write().await;

    match tracker.stop_tracking().await {
        Ok(_) => {
            drop(tracker);
            let status = status_handler(State(state.clone())).await.0;
            Ok(Json(ActionResponse {
                success: true,
                message: "Tracking stopped and logged".to_string(),
                status,
            }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to stop tracking: {}", e),
        )),
    }
}

async fn shutdown_signal() {
    if let Err(err) = signal::ctrl_c().await {
        log::warn!("Failed to listen for shutdown signal: {}", err);
    }
}
