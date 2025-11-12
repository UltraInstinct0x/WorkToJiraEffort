mod config;
mod screenpipe;
mod jira;
mod salesforce;
mod tracker;

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Config;
use tracker::WorkTracker;

#[derive(Parser)]
#[command(name = "work-to-jira-effort")]
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
            println!("Config location: {:?}", std::env::var("HOME").map(|h| format!("{}/.config/WorkToJiraEffort/config.toml", h)));
            Ok(())
        }
        Commands::Check => {
            println!("Loading configuration...");
            let config = Config::load()?;
            println!("Configuration loaded successfully!");
            
            println!("\nChecking service connectivity...");
            let mut tracker = WorkTracker::new(config);
            tracker.check_health().await?;
            
            println!("\nAll checks completed!");
            Ok(())
        }
        Commands::Start => {
            println!("Starting work time tracker...");
            let config = Config::load()?;
            let interval = config.tracking.poll_interval_secs;
            
            let mut tracker = WorkTracker::new(config);
            
            println!("Checking service health before starting...");
            tracker.check_health().await?;
            
            println!("\nStarting continuous tracking (polling every {} seconds)...", interval);
            println!("Press Ctrl+C to stop");
            
            tracker.run(interval).await?;
            Ok(())
        }
    }
}

