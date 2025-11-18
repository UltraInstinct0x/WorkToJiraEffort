use crate::{config::Config, tracker::WorkTracker};
use anyhow::{Context, Result};
use axum::{
    extract::State,
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

    // Start tracker loop in the background
    {
        let tracker_issue_override = Arc::clone(&issue_override);
        let mut tracker = WorkTracker::new(config.clone(), tracker_issue_override);
        let interval = config.tracking.poll_interval_secs;

        tokio::spawn(async move {
            if let Err(err) = tracker.run(interval).await {
                log::error!("Tracker daemon exited with error: {}", err);
            }
        });
    }

    let state = Arc::new(DaemonState { issue_override });

    let app = Router::new()
        .route("/status", get(status_handler))
        .route("/issue", post(issue_override_handler))
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
    issue_override: Arc<RwLock<Option<String>>>,
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
