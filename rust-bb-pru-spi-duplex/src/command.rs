use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::process::Command;

pub struct CommandExecutor;

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

    #[allow(dead_code)]
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
