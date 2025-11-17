use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu},
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

    // On macOS, we MUST activate NSApplication to show menubar icons
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

        // Set activation policy to Accessory (menubar only, no Dock icon)
        app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory);
        println!("✓ NSApplication activated with Accessory policy");
    }

    let state = Arc::new(Mutex::new(AppState {
        daemon_process: None,
    }));

    // Start daemon in BACKGROUND THREAD to not block main thread
    let state_for_daemon = Arc::clone(&state);
    thread::spawn(move || {
        if let Err(e) = start_daemon(&state_for_daemon) {
            eprintln!("Failed to start daemon: {}", e);
        }
    });

    println!("Creating tray icon...");

    // Create tray icon
    let tray_icon = create_tray_icon()
        .context("Failed to create tray icon")?;

    println!("Tray icon created successfully!");

    // Wait briefly for daemon to start, but don't block
    thread::sleep(Duration::from_millis(500));

    // Create initial menu and track menu item IDs
    println!("Creating menu...");
    let menu_items = create_menu(&tray_icon)?;

    println!("WorkToJiraEffort menubar app started!");
    println!("Daemon running on port {}", DAEMON_PORT);
    println!("Look for the blue icon in your menubar (top-right corner)");

    // Keep references alive
    let _tray_icon = tray_icon;
    let _menu_items = menu_items;

    // Start menu event listener
    let menu_channel = MenuEvent::receiver();

    println!("Event loop starting...");

    // Handle menu events with NON-BLOCKING timeout to prevent freezing
    // AND pump the macOS event loop so the menubar icon appears
    loop {
        // Process macOS events to make menubar icon visible AND responsive
        #[cfg(target_os = "macos")]
        unsafe {
            use cocoa::base::id;
            use objc::runtime::Class;
            use objc::{msg_send, sel, sel_impl};

            let _pool: id = msg_send![Class::get("NSAutoreleasePool").unwrap(), new];

            // Get current run loop
            let run_loop_class = Class::get("NSRunLoop").unwrap();
            let run_loop: id = msg_send![run_loop_class, currentRunLoop];

            // Create date 100ms in the future for smoother event processing
            let date_class = Class::get("NSDate").unwrap();
            let date: id = msg_send![date_class, dateWithTimeIntervalSinceNow: 0.1f64];

            // Run the run loop in common modes to handle UI events
            let mode: id = msg_send![Class::get("NSString").unwrap(), stringWithUTF8String: "kCFRunLoopCommonModes\0".as_ptr()];
            let _: () = msg_send![run_loop, runMode:mode beforeDate:date];
        }

        // Check for menu events with longer timeout to reduce CPU usage
        match menu_channel.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                println!("Menu event received: {:?}", event.id);
                if let Err(e) = handle_menu_event(event, &state, &_tray_icon, &_menu_items) {
                    log::error!("Error handling menu event: {}", e);
                }
            }
            Err(e) if e.is_timeout() => {
                // Normal - just keep looping
            }
            Err(_) => {
                eprintln!("Menu event channel disconnected");
                break;
            }
        }
    }

    Ok(())
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
    println!("Generating icon image...");
    let icon = create_icon_image();
    println!("Icon image generated (22x22 RGBA)");

    println!("Building tray icon...");
    let tray_icon = TrayIconBuilder::new()
        .with_tooltip("WorkToJiraEffort")
        .with_icon(icon)
        .build()
        .context("TrayIconBuilder failed")?;

    println!("TrayIcon built successfully!");

    Ok(tray_icon)
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

fn create_menu(tray_icon: &TrayIcon) -> Result<MenuItems> {
    let menu = Menu::new();

    // macOS REQUIRES all items to be in a Submenu, not directly in root Menu
    let submenu = Submenu::new("Menu", true);

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
    submenu.append(&status_item)?;
    submenu.append(&PredefinedMenuItem::separator())?;

    // Refresh status
    let refresh = MenuItem::new("Refresh Status", true, None);
    let refresh_id = refresh.id().clone();
    submenu.append(&refresh)?;
    submenu.append(&PredefinedMenuItem::separator())?;

    // Common issue shortcuts
    let proj_123 = MenuItem::new("Set: PROJ-123", true, None);
    let proj_123_id = proj_123.id().clone();
    submenu.append(&proj_123)?;

    let proj_456 = MenuItem::new("Set: PROJ-456", true, None);
    let proj_456_id = proj_456.id().clone();
    submenu.append(&proj_456)?;

    let proj_789 = MenuItem::new("Set: PROJ-789", true, None);
    let proj_789_id = proj_789.id().clone();
    submenu.append(&proj_789)?;

    submenu.append(&PredefinedMenuItem::separator())?;

    // Clear override
    let clear = MenuItem::new("Clear Override", true, None);
    let clear_id = clear.id().clone();
    submenu.append(&clear)?;

    submenu.append(&PredefinedMenuItem::separator())?;

    // Quit
    let quit = MenuItem::new("Quit", true, None);
    let quit_id = quit.id().clone();
    submenu.append(&quit)?;

    // Add submenu to root menu (macOS requirement)
    menu.append(&submenu)?;

    // Set menu WITHOUT cloning
    tray_icon.set_menu(Some(Box::new(menu)));

    let menu_items = MenuItems {
        proj_123: proj_123_id,
        proj_456: proj_456_id,
        proj_789: proj_789_id,
        clear: clear_id,
        refresh: refresh_id,
        quit: quit_id,
    };

    Ok(menu_items)
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
