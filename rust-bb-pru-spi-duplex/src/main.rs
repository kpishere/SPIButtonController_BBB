mod config;
mod spi;
mod command;
mod daemon;

use anyhow::{Context, Result};
use log::{info, error};
use std::fs;
use std::path::PathBuf;
use tokio::signal::unix::{signal, SignalKind};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    init_logger();

    // Parse command line arguments
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/etc/spi-button-controller/config.yaml".to_string());

    info!("SPI Button Controller starting...");
    info!("Loading configuration from: {}", config_path);

    // Load configuration
    let config_content = fs::read_to_string(&config_path)
        .context(format!("Failed to read config file: {}", config_path))?;
    let config: config::Config = serde_yaml::from_str(&config_content)
        .context("Failed to parse configuration file")?;

    info!("Configuration loaded successfully");

    // Validate SPI device
    let spi_device_path = &config.spi.device;
    if !PathBuf::from(spi_device_path).exists() {
        error!("SPI device not found: {}", spi_device_path);
        return Err(anyhow::anyhow!("SPI device not found: {}", spi_device_path));
    }

    // Create daemon
    let mut daemon = daemon::Daemon::new(config)?;

    // Setup signal handling via tokio
    let mut sigterm = signal(SignalKind::terminate()).context("Failed to setup SIGTERM handler")?;
    let mut sigint = signal(SignalKind::interrupt()).context("Failed to setup SIGINT handler")?;
    let mut sighup = signal(SignalKind::hangup()).context("Failed to setup SIGHUP handler")?;

    info!("Daemon started successfully");

    loop {
        tokio::select! {
            result = daemon.poll() => {
                if let Err(e) = result {
                    error!("Daemon poll error: {}", e);
                    return Err(e);
                }
            }
            _ = sigterm.recv() => {
                info!("Received SIGTERM, shutting down gracefully");
                break;
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, shutting down gracefully");
                break;
            }
            _ = sighup.recv() => {
                info!("Received SIGHUP, reloading configuration");
                let config_content = fs::read_to_string(&config_path)?;
                let new_config: config::Config = serde_yaml::from_str(&config_content)?;
                daemon.reload_config(new_config)?;
                info!("Configuration reloaded successfully");
            }
        }
    }

    info!("SPI Button Controller shutdown complete");
    Ok(())
}

fn init_logger() {
    // Use `env_logger` for logging. Systemd/journald will capture stdout/stderr.
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
}
