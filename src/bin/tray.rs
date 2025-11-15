use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
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

// Menu item IDs
struct MenuItems {
    proj_123: MenuId,
    proj_456: MenuId,
    proj_789: MenuId,
    clear: MenuId,
    refresh: MenuId,
    quit: MenuId,
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

    // Create initial menu and track menu item IDs
    let menu_items = create_menu(&tray_icon)?;

    println!("WorkToJiraEffort menubar app started!");
    println!("Daemon running on port {}", DAEMON_PORT);

    // Start menu event listener
    let menu_channel = MenuEvent::receiver();
    let state_clone = Arc::clone(&state);

    // Handle menu events
    loop {
        if let Ok(event) = menu_channel.recv() {
            if let Err(e) = handle_menu_event(event, &state_clone, &tray_icon, &menu_items) {
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

fn create_menu(tray_icon: &TrayIcon) -> Result<MenuItems> {
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
    let refresh_id = refresh.id().clone();
    menu.append(&refresh)?;
    menu.append(&PredefinedMenuItem::separator())?;

    // Common issue shortcuts
    let proj_123 = MenuItem::new("Set: PROJ-123", true, None);
    let proj_123_id = proj_123.id().clone();
    menu.append(&proj_123)?;

    let proj_456 = MenuItem::new("Set: PROJ-456", true, None);
    let proj_456_id = proj_456.id().clone();
    menu.append(&proj_456)?;

    let proj_789 = MenuItem::new("Set: PROJ-789", true, None);
    let proj_789_id = proj_789.id().clone();
    menu.append(&proj_789)?;

    menu.append(&PredefinedMenuItem::separator())?;

    // Clear override
    let clear = MenuItem::new("Clear Override", true, None);
    let clear_id = clear.id().clone();
    menu.append(&clear)?;

    menu.append(&PredefinedMenuItem::separator())?;

    // Quit
    let quit = MenuItem::new("Quit", true, None);
    let quit_id = quit.id().clone();
    menu.append(&quit)?;

    tray_icon.set_menu(Some(Box::new(menu)));

    Ok(MenuItems {
        proj_123: proj_123_id,
        proj_456: proj_456_id,
        proj_789: proj_789_id,
        clear: clear_id,
        refresh: refresh_id,
        quit: quit_id,
    })
}

fn handle_menu_event(
    event: MenuEvent,
    state: &Arc<Mutex<AppState>>,
    tray_icon: &TrayIcon,
    menu_items: &MenuItems,
) -> Result<()> {
    let id = event.id();

    if id == &menu_items.proj_123 {
        println!("Setting issue override to: PROJ-123");
        match set_issue_override(Some("PROJ-123".to_string())) {
            Ok(_) => {
                println!("Issue override set to: PROJ-123");
                let _ = create_menu(tray_icon)?;
            }
            Err(e) => {
                log::error!("Failed to set issue override: {}", e);
            }
        }
    } else if id == &menu_items.proj_456 {
        println!("Setting issue override to: PROJ-456");
        match set_issue_override(Some("PROJ-456".to_string())) {
            Ok(_) => {
                println!("Issue override set to: PROJ-456");
                let _ = create_menu(tray_icon)?;
            }
            Err(e) => {
                log::error!("Failed to set issue override: {}", e);
            }
        }
    } else if id == &menu_items.proj_789 {
        println!("Setting issue override to: PROJ-789");
        match set_issue_override(Some("PROJ-789".to_string())) {
            Ok(_) => {
                println!("Issue override set to: PROJ-789");
                let _ = create_menu(tray_icon)?;
            }
            Err(e) => {
                log::error!("Failed to set issue override: {}", e);
            }
        }
    } else if id == &menu_items.clear {
        println!("Clearing issue override");
        match set_issue_override(None) {
            Ok(_) => {
                println!("Issue override cleared");
                let _ = create_menu(tray_icon)?;
            }
            Err(e) => {
                log::error!("Failed to clear issue override: {}", e);
            }
        }
    } else if id == &menu_items.refresh {
        println!("Refreshing status...");
        let _ = create_menu(tray_icon)?;
    } else if id == &menu_items.quit {
        println!("Quitting...");
        // Kill daemon if we started it
        let mut state = state.lock().unwrap();
        if let Some(mut child) = state.daemon_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        std::process::exit(0);
    }

    Ok(())
}
