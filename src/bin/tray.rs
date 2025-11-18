use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
    TrayIconBuilder, TrayIconEvent,
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

// Menu item IDs to track which item was clicked
struct MenuIds {
    refresh: MenuId,
    proj_123: MenuId,
    proj_456: MenuId,
    proj_789: MenuId,
    clear: MenuId,
    quit: MenuId,
}

#[derive(Debug)]
enum UserEvent {
    TrayIconEvent(TrayIconEvent),
    MenuEvent(MenuEvent),
}

fn main() -> Result<()> {
    env_logger::init();

    // On macOS, activate NSApplication
    #[cfg(target_os = "macos")]
    unsafe {
        use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicy};
        use cocoa::base::nil;

        println!("Initializing macOS NSApplication...");
        let app = NSApp();
        if app == nil {
            eprintln!("ERROR: Failed to get NSApplication!");
            return Err(anyhow::anyhow!("NSApplication initialization failed"));
        }

        app.setActivationPolicy_(
            NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory,
        );
        println!("✓ NSApplication activated with Accessory policy");
    }

    let state = Arc::new(Mutex::new(AppState {
        daemon_process: None,
    }));

    // Start daemon in background thread
    let state_for_daemon = Arc::clone(&state);
    thread::spawn(move || {
        if let Err(e) = start_daemon(&state_for_daemon) {
            eprintln!("Failed to start daemon: {}", e);
        }
    });

    // Wait briefly for daemon to start
    thread::sleep(Duration::from_millis(500));

    println!("Creating event loop...");
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    // Set up event handlers to forward tray events to the event loop
    let proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::TrayIconEvent(event));
    }));

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::MenuEvent(event));
    }));

    println!("Creating tray icon...");

    // Create tray icon
    let icon = create_icon_image();
    let tray_icon = TrayIconBuilder::new()
        .with_tooltip("WorkToJiraEffort")
        .with_icon(icon)
        .build()
        .context("Failed to create tray icon")?;

    println!("Creating menu...");
    let (menu, menu_ids) = create_menu()?;
    let menu_ids = Arc::new(Mutex::new(menu_ids));

    // Set menu on tray icon
    tray_icon.set_menu(Some(Box::new(menu)));

    println!("WorkToJiraEffort menubar app started!");
    println!("Daemon running on port {}", DAEMON_PORT);
    println!("Look for the blue icon in your menubar (top-right corner)");

    let state_clone = Arc::clone(&state);

    // Run event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                println!("Event loop initialized");
            }
            Event::UserEvent(UserEvent::TrayIconEvent(event)) => {
                println!("Tray icon event: {:?}", event);
            }
            Event::UserEvent(UserEvent::MenuEvent(event)) => {
                println!("Menu event received: {:?}", event.id);
                if let Err(e) = handle_menu_event(event, &state_clone, &tray_icon, &menu_ids) {
                    log::error!("Error handling menu event: {}", e);
                }
            }
            _ => {}
        }
    });
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

fn create_icon_image() -> tray_icon::Icon {
    // Create a 22x22 RGBA icon (blue square) - standard macOS menubar size
    let size = 22;
    let mut rgba = Vec::with_capacity(size * size * 4);
    for _y in 0..size {
        for _x in 0..size {
            rgba.push(0x41); // R
            rgba.push(0x69); // G
            rgba.push(0xE1); // B
            rgba.push(0xFF); // A
        }
    }

    tray_icon::Icon::from_rgba(rgba, size as u32, size as u32).expect("Failed to create icon")
}

fn create_menu() -> Result<(Menu, MenuIds)> {
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

    let menu_ids = MenuIds {
        refresh: refresh_id,
        proj_123: proj_123_id,
        proj_456: proj_456_id,
        proj_789: proj_789_id,
        clear: clear_id,
        quit: quit_id,
    };

    Ok((menu, menu_ids))
}

fn recreate_menu(tray_icon: &tray_icon::TrayIcon, menu_ids: &Arc<Mutex<MenuIds>>) -> Result<()> {
    let (new_menu, new_menu_ids) = create_menu()?;
    tray_icon.set_menu(Some(Box::new(new_menu)));

    // Update stored menu IDs
    let mut ids = menu_ids.lock().unwrap();
    *ids = new_menu_ids;

    Ok(())
}

fn handle_menu_event(
    event: MenuEvent,
    state: &Arc<Mutex<AppState>>,
    tray_icon: &tray_icon::TrayIcon,
    menu_ids: &Arc<Mutex<MenuIds>>,
) -> Result<()> {
    let event_id = event.id();

    // Lock menu_ids to compare
    let ids = menu_ids.lock().unwrap();

    // Check which menu item was clicked using stored IDs
    if event_id == &ids.proj_123 {
        drop(ids); // Release lock before API calls
        println!("Setting issue override to: PROJ-123");
        match set_issue_override(Some("PROJ-123".to_string())) {
            Ok(_) => {
                println!("Issue override set to: PROJ-123");
                recreate_menu(tray_icon, menu_ids)?;
            }
            Err(e) => {
                log::error!("Failed to set issue override: {}", e);
            }
        }
    } else if event_id == &ids.proj_456 {
        drop(ids);
        println!("Setting issue override to: PROJ-456");
        match set_issue_override(Some("PROJ-456".to_string())) {
            Ok(_) => {
                println!("Issue override set to: PROJ-456");
                recreate_menu(tray_icon, menu_ids)?;
            }
            Err(e) => {
                log::error!("Failed to set issue override: {}", e);
            }
        }
    } else if event_id == &ids.proj_789 {
        drop(ids);
        println!("Setting issue override to: PROJ-789");
        match set_issue_override(Some("PROJ-789".to_string())) {
            Ok(_) => {
                println!("Issue override set to: PROJ-789");
                recreate_menu(tray_icon, menu_ids)?;
            }
            Err(e) => {
                log::error!("Failed to set issue override: {}", e);
            }
        }
    } else if event_id == &ids.clear {
        drop(ids);
        println!("Clearing issue override");
        match set_issue_override(None) {
            Ok(_) => {
                println!("Issue override cleared");
                recreate_menu(tray_icon, menu_ids)?;
            }
            Err(e) => {
                log::error!("Failed to clear issue override: {}", e);
            }
        }
    } else if event_id == &ids.refresh {
        drop(ids);
        println!("Refreshing status...");
        recreate_menu(tray_icon, menu_ids)?;
    } else if event_id == &ids.quit {
        drop(ids);
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
