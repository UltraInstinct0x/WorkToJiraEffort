use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use tracing::{info, warn};

/// Manages the embedded Screenpipe server lifecycle as a subprocess
pub struct ScreenpipeManager {
    process: Option<Child>,
    data_dir: PathBuf,
}

impl ScreenpipeManager {
    pub fn new() -> Self {
        Self {
            process: None,
            data_dir: PathBuf::new(),
        }
    }

    /// Start the embedded Screenpipe server as a subprocess
    pub async fn start(&mut self, data_dir: PathBuf, port: u16) -> Result<()> {
        info!("Starting embedded Screenpipe server on port {}", port);

        self.data_dir = data_dir.clone();

        // Ensure data directory exists
        std::fs::create_dir_all(&data_dir)
            .context("Failed to create Screenpipe data directory")?;

        // Try to find screenpipe binary
        let screenpipe_path = self.find_screenpipe_binary()?;
        
        info!("Found Screenpipe binary at: {:?}", screenpipe_path);

        // Start screenpipe process
        let process = Command::new(screenpipe_path)
            .arg("--port")
            .arg(port.to_string())
            .arg("--data-dir")
            .arg(data_dir.to_string_lossy().to_string())
            .arg("--disable-audio")  // Simplify by disabling audio initially
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to start Screenpipe process")?;

        self.process = Some(process);

        // Give the server time to start
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Verify the server is running
        let client = reqwest::Client::new();
        let health_url = format!("http://localhost:{}/health", port);
        
        match client.get(&health_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                info!("Screenpipe server started successfully and is healthy");
                Ok(())
            }
            Ok(resp) => {
                self.stop().await?;
                Err(anyhow::anyhow!(
                    "Screenpipe server health check failed with status: {}",
                    resp.status()
                ))
            }
            Err(e) => {
                self.stop().await?;
                Err(anyhow::anyhow!(
                    "Failed to connect to Screenpipe server: {}",
                    e
                ))
            }
        }
    }

    /// Find the Screenpipe binary in various locations
    fn find_screenpipe_binary(&self) -> Result<PathBuf> {
        // Try multiple locations where screenpipe might be installed
        let possible_paths = vec![
            // In system PATH
            which::which("screenpipe").ok(),
            // Common installation locations
            Some(PathBuf::from("/usr/local/bin/screenpipe")),
            Some(PathBuf::from("/usr/bin/screenpipe")),
            // In user's home directory
            dirs::home_dir().map(|h| h.join(".cargo/bin/screenpipe")),
            dirs::home_dir().map(|h| h.join(".local/bin/screenpipe")),
            // Windows locations
            dirs::home_dir().map(|h| h.join("AppData/Local/screenpipe/screenpipe.exe")),
            // macOS locations
            Some(PathBuf::from("/Applications/screenpipe.app/Contents/MacOS/screenpipe")),
        ];

        for path_opt in possible_paths {
            if let Some(path) = path_opt {
                if path.exists() {
                    return Ok(path);
                }
            }
        }

        // If not found, try to install it
        self.install_screenpipe()
    }

    /// Install Screenpipe using the install script
    fn install_screenpipe(&self) -> Result<PathBuf> {
        info!("Screenpipe not found, attempting to install...");

        #[cfg(unix)]
        {
            // Download and run the install script
            let output = Command::new("sh")
                .arg("-c")
                .arg("curl -fsSL https://raw.githubusercontent.com/mediar-ai/screenpipe/main/install.sh | sh")
                .output()
                .context("Failed to run Screenpipe install script")?;

            if !output.status.success() {
                return Err(anyhow::anyhow!(
                    "Screenpipe installation failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }

            // Try to find it again
            if let Ok(path) = which::which("screenpipe") {
                return Ok(path);
            }
        }

        #[cfg(windows)]
        {
            // Download and run the Windows install script
            let output = Command::new("powershell")
                .arg("-ExecutionPolicy")
                .arg("Bypass")
                .arg("-Command")
                .arg("iwr https://raw.githubusercontent.com/mediar-ai/screenpipe/main/install.ps1 -useb | iex")
                .output()
                .context("Failed to run Screenpipe install script")?;

            if !output.status.success() {
                return Err(anyhow::anyhow!(
                    "Screenpipe installation failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }

        Err(anyhow::anyhow!(
            "Failed to install Screenpipe automatically. Please install it manually from https://github.com/mediar-ai/screenpipe"
        ))
    }

    /// Stop the embedded Screenpipe server
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping embedded Screenpipe server");

        if let Some(mut process) = self.process.take() {
            #[cfg(unix)]
            {
                // Send SIGTERM to gracefully shutdown
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;
                
                let pid = process.id();
                let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
            }

            #[cfg(windows)]
            {
                // On Windows, just kill the process
                let _ = process.kill();
            }

            // Wait for the process to exit
            match tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                tokio::task::spawn_blocking(move || process.wait()),
            )
            .await
            {
                Ok(Ok(Ok(status))) => {
                    info!("Screenpipe server stopped with status: {}", status);
                }
                Ok(Ok(Err(e))) => {
                    warn!("Error waiting for Screenpipe process: {}", e);
                }
                Ok(Err(e)) => {
                    warn!("Task join error: {}", e);
                }
                Err(_) => {
                    warn!("Screenpipe server shutdown timeout");
                }
            }
        }

        Ok(())
    }

    /// Check if the server is running
    pub fn is_running(&self) -> bool {
        self.process.is_some()
    }
}

impl Drop for ScreenpipeManager {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
        }
    }
}

