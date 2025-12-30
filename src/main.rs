mod config;
mod command;
mod daemon;

use anyhow::{Context, Result};
use log::{info, error};
use std::fs;
use std::path::PathBuf;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use crate::command::EventMessage;
use std::collections::HashMap;
use spibuttonlib::SPIButtonState;

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
    let mut config: config::Config = serde_yaml::from_str(&config_content)
        .context("Failed to parse configuration file")?;

    // Sort by button number & sanity check unique button IDs as ordinal vector number === button ID
    config.buttons.sort_by(|a,b| {a.button.cmp(&b.button)});
    let bcnt: usize = config.buttons.len();
    if bcnt != config.buttons[ bcnt - 1 ].button as usize + 1
    {
        return Err(anyhow::anyhow!("Configuration error for button IDs, they must be consective starting from zero."));
    }

    info!("Configuration loaded successfully");

    // Validate SPI device
    let spi_device_path = &config.spi.device;
    if !PathBuf::from(spi_device_path).exists() {
        error!("SPI device not found: {}", spi_device_path);
        return Err(anyhow::anyhow!("SPI device not found: {}", spi_device_path));
    }

    // Create response queue for Klipper command replies
    let (resp_tx, mut resp_rx) = mpsc::channel::<EventMessage>(32);

    // map request_id -> trigger_info for correlation
    let mut pending: HashMap<u32, String> = HashMap::new();

    // Create daemon and provide response sender
    let mut daemon = daemon::Daemon::new(config, Some(resp_tx))?;

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
            // Klipper command messages (issued & responses)
            maybe_msg = resp_rx.recv() => {
                if let Some(msg) = maybe_msg {
                    match msg {
                        EventMessage::Issued { request_id, trigger_button } => {
                            // persist mapping for later correlation
                            pending.insert(request_id.clone(), trigger_button.clone());
                            info!("Tracked issued request id={} triger_button={}", request_id, trigger_button);
                        }
                        EventMessage::Response(resp) => {
                            // correlate with original trigger
                            if let Some(button) = pending.remove(&resp.request_id) {
                                let mut final_button_status = SPIButtonState::Off;
                                let button_u8 = button.parse::<u8>().unwrap();
                                info!("Klipper response id={} correlated_to={} success={} status={:?} body={:?}"
                                    , resp.request_id, button, resp.success, resp.status, resp.body);
                                if resp.success {
                                } else {
                                    match resp.status {
                                        Some(msg) => {
                                            match msg.as_ref() {
                                                "empty_response" => {
                                                    // OK case: restart
                                                },
                                                _ => {
                                                    // Presumed error case
                                                    final_button_status = SPIButtonState::Flash2;
                                                },
                                            }
                                        }
                                        _ => {
                                            // error case, no status
                                            final_button_status = SPIButtonState::Flash2;
                                        }
                                    }
                                }
                                daemon.set_button_state(button_u8, final_button_status);
                            } else {
                                info!("Klipper response id={} (no matching issue found) success={} status={:?} body={:?}", resp.request_id, resp.success, resp.status, resp.body);
                            }
                            // TODO: Set value on button
                        }
                    }
                }
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
