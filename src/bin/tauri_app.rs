use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Manager, State};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconEvent;

const DAEMON_PORT: u16 = 8787;
const DAEMON_URL: &str = "http://127.0.0.1:8787";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatusResponse {
    version: String,
    issue_override: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IssueRequest {
    issue_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActivitySummary {
    current_issue: Option<String>,
    total_tracked_today: String,
    is_tracking: bool,
}

struct AppState {
    daemon_url: String,
}

#[tauri::command]
async fn get_status(state: State<'_, Arc<Mutex<AppState>>>) -> Result<StatusResponse, String> {
    let daemon_url = {
        let state = state.lock().unwrap();
        state.daemon_url.clone()
    };

    let client = reqwest::Client::new();

    client
        .get(&format!("{}/status", daemon_url))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("Failed to get status: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse status: {}", e))
}

#[tauri::command]
async fn set_issue_override(
    issue_key: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<StatusResponse, String> {
    let daemon_url = {
        let state = state.lock().unwrap();
        state.daemon_url.clone()
    };

    let client = reqwest::Client::new();

    client
        .post(&format!("{}/issue", daemon_url))
        .json(&IssueRequest { issue_key })
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("Failed to set issue: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

#[tauri::command]
async fn get_activity_summary(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ActivitySummary, String> {
    // Get current status
    let status = get_status(state).await?;

    // TODO: In the future, query actual tracking data from daemon
    // For now, return basic info
    let is_tracking = status.issue_override.is_some();
    Ok(ActivitySummary {
        current_issue: status.issue_override,
        total_tracked_today: "0h 0m".to_string(),
        is_tracking,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Initialize app state
            let state = Arc::new(Mutex::new(AppState {
                daemon_url: DAEMON_URL.to_string(),
            }));
            app.manage(state);

            // Set up system tray menu
            let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let tray = app.tray_by_id("main").expect("Failed to get tray");
            tray.set_menu(Some(menu))?;

            tray.on_menu_event(|app, event| {
                match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                }
            });

            tray.on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click { .. } = event {
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            set_issue_override,
            get_activity_summary
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}
