use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, TrayIconEvent},
    Manager, State, WindowEvent,
};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecentIssue {
    key: String,
    title: Option<String>,
    total_time: String,
    last_used: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DailySummary {
    total_time: String,
    issues: Vec<IssueTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IssueTime {
    issue_key: String,
    duration: String,
    percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NotificationPrefs {
    enabled: bool,
    frequency: String, // "immediate", "hourly", "daily"
}

struct AppState {
    daemon_url: String,
    daemon_process: Option<std::process::Child>,
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

#[tauri::command]
async fn get_recent_issues() -> Result<Vec<RecentIssue>, String> {
    // TODO: Query actual recent issues from daemon/database
    // For now, return mock data
    Ok(vec![
        RecentIssue {
            key: "PROJ-123".to_string(),
            title: Some("Implement new feature".to_string()),
            total_time: "2h 30m".to_string(),
            last_used: "2025-11-19T10:30:00Z".to_string(),
        },
        RecentIssue {
            key: "PROJ-456".to_string(),
            title: Some("Fix critical bug".to_string()),
            total_time: "1h 15m".to_string(),
            last_used: "2025-11-19T09:00:00Z".to_string(),
        },
        RecentIssue {
            key: "PROJ-789".to_string(),
            title: Some("Code review".to_string()),
            total_time: "45m".to_string(),
            last_used: "2025-11-18T16:20:00Z".to_string(),
        },
    ])
}

#[tauri::command]
async fn get_daily_summary() -> Result<DailySummary, String> {
    // TODO: Query actual daily summary from daemon/database
    // For now, return mock data
    Ok(DailySummary {
        total_time: "4h 30m".to_string(),
        issues: vec![
            IssueTime {
                issue_key: "PROJ-123".to_string(),
                duration: "2h 30m".to_string(),
                percentage: 55.56,
            },
            IssueTime {
                issue_key: "PROJ-456".to_string(),
                duration: "1h 15m".to_string(),
                percentage: 27.78,
            },
            IssueTime {
                issue_key: "PROJ-789".to_string(),
                duration: "45m".to_string(),
                percentage: 16.67,
            },
        ],
    })
}

#[tauri::command]
async fn get_notification_prefs() -> Result<NotificationPrefs, String> {
    // TODO: Load actual preferences from config file or database
    // For now, return mock data
    Ok(NotificationPrefs {
        enabled: true,
        frequency: "hourly".to_string(),
    })
}

#[tauri::command]
async fn set_notification_prefs(prefs: NotificationPrefs) -> Result<(), String> {
    // TODO: Persist preferences to config file or database
    // For now, just validate and return success

    // Validate frequency
    let valid_frequencies = ["immediate", "hourly", "daily"];
    if !valid_frequencies.contains(&prefs.frequency.as_str()) {
        return Err(format!(
            "Invalid frequency '{}'. Must be one of: immediate, hourly, daily",
            prefs.frequency
        ));
    }

    println!(
        "Notification preferences updated: enabled={}, frequency={}",
        prefs.enabled, prefs.frequency
    );

    Ok(())
}

fn start_daemon() -> Result<std::process::Child> {
    // Check if daemon is already running
    let client = reqwest::blocking::Client::new();
    if client
        .get(&format!("{}/status", DAEMON_URL))
        .timeout(Duration::from_secs(1))
        .send()
        .is_ok()
    {
        println!("Daemon already running");
        // If already running, we can't easily get the child handle, so we return a dummy or handle this differently.
        // For simplicity in this refactor, we'll assume if it's running we don't own it,
        // but to satisfy the type signature we might need to spawn a dummy or change logic.
        // However, for a robust app, we should probably kill existing or just connect.
        // Let's try to start it anyway if it fails, or just return error if running?
        // Better: just try to spawn. If port is taken, it will fail or handle it.
    }

    println!("Starting daemon...");

    // Get the path to the main binary
    let exe_path = std::env::current_exe()?;
    // In the bundle, the daemon binary should be in the same folder as the UI binary
    let daemon_exe = exe_path.parent().unwrap().join("work-to-jira-effort");

    #[cfg(target_os = "macos")]
    let child = {
        use std::os::unix::process::CommandExt;
        Command::new(daemon_exe)
            .args(["daemon", "--port", &DAEMON_PORT.to_string()])
            .env("RUST_LOG", "info")
            .env("WORK_TO_JIRA_NO_DOCK", "1") // Signal to daemon to not show in dock
            .process_group(0) // Create new process group
            .spawn()
            .context("Failed to start daemon process")?
    };

    #[cfg(not(target_os = "macos"))]
    let child = Command::new(daemon_exe)
        .args(["daemon", "--port", &DAEMON_PORT.to_string()])
        .env("RUST_LOG", "info")
        .spawn()
        .context("Failed to start daemon process")?;

    Ok(child)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Start daemon
            let daemon_process = match start_daemon() {
                Ok(child) => Some(child),
                Err(e) => {
                    eprintln!("Failed to start daemon: {}", e);
                    None
                }
            };

            // Initialize app state
            let state = Arc::new(Mutex::new(AppState {
                daemon_url: DAEMON_URL.to_string(),
                daemon_process,
            }));
            app.manage(state);

            // Set up system tray menu
            let show_item = MenuItem::with_id(app, "show", "Open Dashboard", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;

            let menu = Menu::with_items(app, &[&show_item, &separator, &quit_item])?;

            let tray = app.tray_by_id("main").expect("Failed to get tray");
            tray.set_menu(Some(menu))?;

            tray.on_menu_event(|app, event| match event.id().as_ref() {
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
            });

            tray.on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    ..
                } = event
                {
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            set_issue_override,
            get_activity_summary,
            get_recent_issues,
            get_daily_summary,
            get_notification_prefs,
            set_notification_prefs
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                // Kill daemon on exit
                let state = app_handle.state::<Arc<Mutex<AppState>>>();
                let mut state = state.lock().unwrap();
                if let Some(mut child) = state.daemon_process.take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        });
}

fn main() {
    run();
}
