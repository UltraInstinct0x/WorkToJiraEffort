use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder,
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

struct AppState {
    daemon_process: Option<Child>,
}

fn main() -> Result<()> {
    env_logger::init();

    let state = Arc::new(Mutex::new(AppState {
        daemon_process: None,
    }));

    // Try to start daemon
    start_daemon(&state)?;

    // Wait a bit for daemon to start
    thread::sleep(Duration::from_secs(2));

    // Create tray icon
    let tray_icon = create_tray_icon()?;

    // Create initial menu
    let menu = create_menu()?;
    tray_icon.set_menu(Some(Box::new(menu)));

    println!("WorkToJiraEffort menubar app started!");
    println!("Daemon running on port {}", DAEMON_PORT);

    // Start menu event listener
    let menu_channel = MenuEvent::receiver();
    let state_clone = Arc::clone(&state);

    // Handle menu events
    loop {
        if let Ok(event) = menu_channel.recv() {
            if let Err(e) = handle_menu_event(event, &state_clone, &tray_icon) {
                log::error!("Error handling menu event: {}", e);
            }
        }
    }
}

fn start_daemon(state: &Arc<Mutex<AppState>>) -> Result<()> {
    // Check if daemon is already running
    if check_daemon_health().is_ok() {
        println!("Daemon already running");
        return Ok(());
    }

    println!("Starting daemon...");

    // Get the path to the main binary
    let exe_path = std::env::current_exe()?;
    let daemon_exe = exe_path.parent().unwrap().join("work-to-jira-effort");

    let child = Command::new(daemon_exe)
        .args(["daemon", "--port", &DAEMON_PORT.to_string()])
        .env("RUST_LOG", "info")
        .spawn()
        .context("Failed to start daemon process")?;

    let mut state = state.lock().unwrap();
    state.daemon_process = Some(child);

    Ok(())
}

fn check_daemon_health() -> Result<()> {
    let client = reqwest::blocking::Client::new();
    client
        .get(&format!("{}/status", DAEMON_URL))
        .timeout(Duration::from_secs(2))
        .send()?;
    Ok(())
}

fn get_status() -> Result<StatusResponse> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(&format!("{}/status", DAEMON_URL))
        .timeout(Duration::from_secs(5))
        .send()?
        .json()?;
    Ok(response)
}

fn set_issue_override(issue_key: Option<String>) -> Result<StatusResponse> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&format!("{}/issue", DAEMON_URL))
        .json(&IssueRequest { issue_key })
        .timeout(Duration::from_secs(5))
        .send()?
        .json()?;
    Ok(response)
}

fn create_tray_icon() -> Result<TrayIcon> {
    // Create a simple icon (a colored square)
    let icon = create_icon_image();

    let tray_icon = TrayIconBuilder::new()
        .with_tooltip("WorkToJiraEffort")
        .with_icon(icon)
        .build()?;

    Ok(tray_icon)
}

fn create_icon_image() -> tray_icon::Icon {
    // Create a 32x32 RGBA icon (blue square)
    let mut rgba = Vec::with_capacity(32 * 32 * 4);
    for _y in 0..32 {
        for _x in 0..32 {
            rgba.push(0x41); // R
            rgba.push(0x69); // G
            rgba.push(0xE1); // B
            rgba.push(0xFF); // A
        }
    }

    tray_icon::Icon::from_rgba(rgba, 32, 32).expect("Failed to create icon")
}

fn create_menu() -> Result<Menu> {
    let menu = Menu::new();

    // Status item - get current status from daemon
    let status_text = match get_status() {
        Ok(status) => {
            if let Some(ref issue) = status.issue_override {
                format!("Current: {}", issue)
            } else {
                "No override set".to_string()
            }
        }
        Err(_) => "Status: Unknown".to_string(),
    };

    let status_item = MenuItem::new(status_text, false, None);
    menu.append(&status_item)?;
    menu.append(&PredefinedMenuItem::separator())?;

    // Refresh status
    let refresh = MenuItem::new("Refresh Status", true, None);
    menu.append(&refresh)?;
    menu.append(&PredefinedMenuItem::separator())?;

    // Common issue shortcuts
    let proj_123 = MenuItem::new("Set: PROJ-123", true, None);
    let proj_456 = MenuItem::new("Set: PROJ-456", true, None);
    let proj_789 = MenuItem::new("Set: PROJ-789", true, None);
    menu.append(&proj_123)?;
    menu.append(&proj_456)?;
    menu.append(&proj_789)?;

    menu.append(&PredefinedMenuItem::separator())?;

    // Clear override
    let clear = MenuItem::new("Clear Override", true, None);
    menu.append(&clear)?;

    menu.append(&PredefinedMenuItem::separator())?;

    // Quit
    let quit = MenuItem::new("Quit", true, None);
    menu.append(&quit)?;

    Ok(menu)
}

fn handle_menu_event(
    event: MenuEvent,
    state: &Arc<Mutex<AppState>>,
    tray_icon: &TrayIcon,
) -> Result<()> {
    let id = event.id();

    // Get the menu to find which item was clicked
    let menu = tray_icon.menu().unwrap();
    let items = menu.items();
    let clicked_item = items.iter().find(|item| {
        if let Some(menu_item) = item.as_menuitem() {
            menu_item.id() == &id
        } else if let Some(pred_item) = item.as_predefined_menuitem() {
            pred_item.id() == &id
        } else {
            false
        }
    });

    if let Some(item) = clicked_item {
        if let Some(menu_item) = item.as_menuitem() {
            let text = menu_item.text();

            if text.starts_with("Set: ") {
                let issue = text.strip_prefix("Set: ").unwrap().to_string();
                println!("Setting issue override to: {}", issue);
                match set_issue_override(Some(issue.clone())) {
                    Ok(_) => {
                        println!("Issue override set to: {}", issue);
                        // Recreate menu with updated status
                        let new_menu = create_menu()?;
                        tray_icon.set_menu(Some(Box::new(new_menu)));
                    }
                    Err(e) => {
                        log::error!("Failed to set issue override: {}", e);
                    }
                }
            } else if text == "Clear Override" {
                println!("Clearing issue override");
                match set_issue_override(None) {
                    Ok(_) => {
                        println!("Issue override cleared");
                        // Recreate menu with updated status
                        let new_menu = create_menu()?;
                        tray_icon.set_menu(Some(Box::new(new_menu)));
                    }
                    Err(e) => {
                        log::error!("Failed to clear issue override: {}", e);
                    }
                }
            } else if text == "Refresh Status" {
                println!("Refreshing status...");
                // Recreate menu with updated status
                let new_menu = create_menu()?;
                tray_icon.set_menu(Some(Box::new(new_menu)));
            } else if text == "Quit" {
                println!("Quitting...");
                // Kill daemon if we started it
                let mut state = state.lock().unwrap();
                if let Some(mut child) = state.daemon_process.take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                std::process::exit(0);
            }
        }
    }

    Ok(())
}
