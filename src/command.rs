use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::process::Command;
use serde_json::Value as JsonValue;
use tokio::sync::mpsc::Sender;
use tokio::net::UnixStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt};

use crate::config::KlipperConfig;

pub struct CommandExecutor;

/// Response pushed into the event response queue when a Klipper command returns
#[derive(Debug, Clone)]
pub struct EventResponse {
    pub request_id: u32,
    pub success: bool,
    pub status: Option<String>,
    pub body: Option<JsonValue>,
}

/// Event messages sent over the event channel. `Issued` is sent when a
/// request is created (so the main loop can persist metadata). `Response`
/// carries the response from Klipper.
#[derive(Debug, Clone)]
pub enum EventMessage {
    Issued { request_id: u32, trigger_info: String },
    Response(EventResponse),
}

impl CommandExecutor {
    pub fn execute(command: &str) -> Result<()> {
        info!("Executing command: {}", command);

        // Execute the command through a shell
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .context(format!("Failed to execute command: {}", command))?;

        if output.status.success() {
            if !output.stdout.is_empty() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                debug!("Command output: {}", stdout);
            }
            info!("Command executed successfully");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                "Command execution failed with status: {:?}. Error: {}",
                output.status, stderr
            );
            Err(anyhow::anyhow!(
                "Command failed with status: {:?}",
                output.status
            ))
        }
    }
/*
    pub fn execute_with_timeout(command: &str, timeout_secs: u64) -> Result<()> {
        info!(
            "Executing command with {} second timeout: {}",
            timeout_secs, command
        );

        let output = Command::new("timeout")
            .arg(timeout_secs.to_string())
            .arg("sh")
            .arg("-c")
            .arg(command)
            .output()
            .context(format!("Failed to execute command: {}", command))?;

        if output.status.success() {
            if !output.stdout.is_empty() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                debug!("Command output: {}", stdout);
            }
            info!("Command executed successfully");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                "Command execution failed with status: {:?}. Error: {}",
                output.status, stderr
            );
            Err(anyhow::anyhow!(
                "Command failed with status: {:?}",
                output.status
            ))
        }
    }
*/
    /// Send a Klipper API command asynchronously via Unix Domain Socket.
    ///
    /// Command string format (simple syntax):
    /// klipper:METHOD|<JSON_PARAMS>
    /// Example: klipper:printer.gcode.script|{"script":"G28"}
    pub async fn send_klipper_command(
        command: &str,
        klipper: &KlipperConfig,
        request_id: u32,
        response_tx: Sender<EventMessage>,
    ) {
        info!("Preparing Klipper command: {}", command);

        // Strip prefix if present
        let payload = command.strip_prefix("klipper:").unwrap_or(command);

        // Split into method and params
        let mut parts = payload.splitn(2, '|');
        let method = parts.next().unwrap_or("");
        let params_str = parts.next().unwrap_or("{}");

        let params_json: JsonValue = match serde_json::from_str(params_str) {
            Ok(v) => v,
            Err(e) => {
                warn!("Failed to parse Klipper params JSON: {}", e);
                let _ = response_tx
                    .send(EventMessage::Response(EventResponse {
                        request_id,
                        success: false,
                        status: Some("invalid_params".to_string()),
                        body: None,
                    }))
                    .await;
                return;
            }
        };

        // Build JSON-RPC like body using provided request_id
        let mut body = serde_json::Map::new();
        body.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
        body.insert("id".to_string(), JsonValue::Number(request_id.into()));
        body.insert("method".to_string(), JsonValue::String(method.to_string()));
        body.insert("params".to_string(), params_json.clone());

        let request_json = serde_json::to_string(&JsonValue::Object(body))
            .unwrap_or_default();
        
        // Attempt to connect to Unix domain socket
        match UnixStream::connect(&klipper.socket_path).await {
            Ok(mut stream) => {
                // Send the request
                if let Err(e) = stream.write_all(request_json.as_bytes()).await {
                    warn!("Failed to write to Unix socket: {}", e);
                    let _ = response_tx
                        .send(EventMessage::Response(EventResponse {
                            request_id,
                            success: false,
                            status: Some(format!("socket_write_error: {}", e)),
                            body: None,
                        }))
                        .await;
                    return;
                }

                // Send ETX (ASCII 0x03) to signal end of request
                if let Err(e) = stream.write_all(&[0x03]).await {
                    warn!("Failed to write ETX to Unix socket: {}", e);
                    let _ = response_tx
                        .send(EventMessage::Response(EventResponse {
                            request_id,
                            success: false,
                            status: Some(format!("socket_write_error: {}", e)),
                            body: None,
                        }))
                        .await;
                    return;
                }

                // Read response
                let mut buffer = vec![0; 4096];
                match stream.read(&mut buffer).await {
                    Ok(n) if n > 0 => {
                        let response_str = String::from_utf8_lossy(&buffer[..n]);
                        match serde_json::from_str::<JsonValue>(&response_str) {
                            Ok(json_response) => {
                                let success = !response_str.contains("\"error\"");
                                let status = if success {
                                    "200".to_string()
                                } else {
                                    "error".to_string()
                                };

                                let _ = response_tx
                                    .send(EventMessage::Response(EventResponse {
                                        request_id,
                                        success,
                                        status: Some(status),
                                        body: Some(json_response),
                                    }))
                                    .await;
                            }
                            Err(e) => {
                                warn!("Failed to parse Klipper response JSON: {}", e);
                                let _ = response_tx
                                    .send(EventMessage::Response(EventResponse {
                                        request_id,
                                        success: false,
                                        status: Some(format!("parse_error: {}", e)),
                                        body: None,
                                    }))
                                    .await;
                            }
                        }
                    }
                    Ok(_) => {
                        warn!("Received empty response from Klipper socket");
                        let _ = response_tx
                            .send(EventMessage::Response(EventResponse {
                                request_id,
                                success: false,
                                status: Some("empty_response".to_string()),
                                body: None,
                            }))
                            .await;
                    }
                    Err(e) => {
                        warn!("Failed to read from Unix socket: {}", e);
                        let _ = response_tx
                            .send(EventMessage::Response(EventResponse {
                                request_id,
                                success: false,
                                status: Some(format!("socket_read_error: {}", e)),
                                body: None,
                            }))
                            .await;
                    }
                }
            }
            Err(e) => {
                warn!("Failed to connect to Klipper Unix socket at {}: {}", klipper.socket_path, e);
                let _ = response_tx
                    .send(EventMessage::Response(EventResponse {
                        request_id,
                        success: false,
                        status: Some(format!("connection_error: {}", e)),
                        body: None,
                    }))
                    .await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_success() {
        let result = CommandExecutor::execute("echo 'test'");
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_failure() {
        let result = CommandExecutor::execute("false");
        assert!(result.is_err());
    }
}
