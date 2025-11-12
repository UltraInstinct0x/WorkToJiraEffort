mod config;
mod jira;
mod salesforce;
mod screenpipe;
mod screenpipe_manager;
mod tracker;

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Config;
use screenpipe_manager::ScreenpipeManager;
use tracker::WorkTracker;
use std::path::PathBuf;
use directories::ProjectDirs;

#[derive(Parser)]
#[command(name = "work-to-jira-effort")]
#[command(version = "0.1.0")]
#[command(about = "Automatically track work time via Screenpipe and log to Jira & Salesforce", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start tracking work time
    Start,
    /// Check configuration and service connectivity
    Check,
    /// Initialize configuration file
    Init,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            println!("Initializing configuration...");
            let config = Config::default();
            config.save()?;
            println!("Configuration file created successfully!");
            println!("Please edit the configuration file with your credentials.");
            println!(
                "Config location: {:?}",
                std::env::var("HOME")
                    .map(|h| format!("{}/.config/WorkToJiraEffort/config.toml", h))
            );
            Ok(())
        }
        Commands::Check => {
            println!("Loading configuration...");
            let config = Config::load()?;
            println!("Configuration loaded successfully!");

            // Get data directory for embedded Screenpipe
            let data_dir = get_data_dir()?;
            
            // Start embedded Screenpipe server
            println!("\nStarting embedded Screenpipe server...");
            let mut screenpipe = ScreenpipeManager::new();
            screenpipe.start(data_dir, 3030).await?;
            
            println!("\nChecking service connectivity...");
            let mut tracker = WorkTracker::new(config);
            tracker.check_health().await?;

            // Stop Screenpipe server
            screenpipe.stop().await?;

            println!("\nAll checks completed!");
            Ok(())
        }
        Commands::Start => {
            println!("Starting work time tracker with embedded Screenpipe...");
            let config = Config::load()?;
            let interval = config.tracking.poll_interval_secs;

            // Get data directory for embedded Screenpipe
            let data_dir = get_data_dir()?;
            
            // Start embedded Screenpipe server
            println!("Starting embedded Screenpipe server...");
            let mut screenpipe = ScreenpipeManager::new();
            screenpipe.start(data_dir, 3030).await?;

            let mut tracker = WorkTracker::new(config);

            println!("Checking service health before starting...");
            tracker.check_health().await?;

            println!(
                "\nStarting continuous tracking (polling every {} seconds)...",
                interval
            );
            println!("Press Ctrl+C to stop");

            // Set up Ctrl+C handler
            let result = tokio::select! {
                res = tracker.run(interval) => res,
                _ = tokio::signal::ctrl_c() => {
                    println!("\nShutdown signal received, stopping...");
                    Ok(())
                }
            };

            // Stop Screenpipe server
            screenpipe.stop().await?;

            result
        }
    }
}

/// Get the data directory for storing Screenpipe data
fn get_data_dir() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "worktojiraeffort", "WorkToJiraEffort")
        .ok_or_else(|| anyhow::anyhow!("Failed to determine project directories"))?;
    
    let data_dir = proj_dirs.data_dir().join("screenpipe");
    std::fs::create_dir_all(&data_dir)?;
    
    Ok(data_dir)
}
