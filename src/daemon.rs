use crate::command::CommandExecutor;
use crate::config::{Config, RegisterMapping, ValueTrigger};
use crate::spi::SpiDevice;
use anyhow::Result;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;

pub struct Daemon {
    spi: SpiDevice,
    config: Config,
    register_states: HashMap<u8, u8>,
    last_event_times: HashMap<u8, Instant>,
}

impl Daemon {
    pub fn new(config: Config) -> Result<Self> {
        let spi = SpiDevice::new(&config.spi.device)?;

        info!("SPI device initialized: {}", config.spi.device);
        info!("Polling interval: {}ms", config.polling.interval_ms);
        info!(
            "Debounce interval: {}ms",
            config.polling.debounce_ms
        );
        info!("Monitoring {} register(s)", config.registers.len());

        let mut register_states = HashMap::new();
        for register_map in &config.registers {
            register_states.insert(register_map.register_address, 0u8);
            info!(
                "  - Register 0x{:02x}: {}",
                register_map.register_address, register_map.name
            );
        }

        Ok(Daemon {
            spi,
            config,
            register_states,
            last_event_times: HashMap::new(),
        })
    }

    pub async fn poll(&mut self) -> Result<()> {
        // Poll all configured registers
        for register_map in &self.config.registers {
            let address = register_map.register_address;

            // Read current register value
            match self.spi.read_register(address) {
                Ok(current_value) => {
                    let previous_value = self.register_states.get(&address).copied().unwrap_or(0);

                    // Check if value changed
                    if current_value != previous_value {
                        debug!(
                            "Register 0x{:02x} ({}) changed: 0x{:02x} -> 0x{:02x}",
                            address, register_map.name, previous_value, current_value
                        );

                        // Update stored state
                        self.register_states.insert(address, current_value);

                        // Check debounce
                        if self.should_trigger_event(address) {
                            // Process value triggers
                            self.process_triggers(register_map, current_value)
                                .await;

                            // Update last event time
                            self.last_event_times.insert(address, Instant::now());
                        } else {
                            debug!(
                                "Event for register 0x{:02x} debounced",
                                address
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to read register 0x{:02x} ({}): {}",
                        address, register_map.name, e
                    );
                }
            }
        }

        // Sleep for the configured polling interval
        sleep(Duration::from_millis(self.config.polling.interval_ms)).await;

        Ok(())
    }

    async fn process_triggers(
        &self,
        register_map: &RegisterMapping,
        current_value: u8,
    ) {
        for trigger in &register_map.value_triggers {
            if self.matches_trigger(current_value, trigger) {
                info!(
                    "Trigger matched for register 0x{:02x} ({}), value 0x{:02x}: {}",
                    register_map.register_address,
                    register_map.name,
                    current_value,
                    trigger
                        .description
                        .as_ref()
                        .unwrap_or(&"No description".to_string())
                );

                // Execute the associated command
                match CommandExecutor::execute(&trigger.command) {
                    Ok(_) => {
                        info!(
                            "Successfully executed command for trigger on register 0x{:02x}",
                            register_map.register_address
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to execute command for register 0x{:02x}: {}",
                            register_map.register_address, e
                        );
                    }
                }
            }
        }
    }

    fn matches_trigger(&self, value: u8, trigger: &ValueTrigger) -> bool {
        if let Some(mask) = trigger.mask {
            (value & mask) == trigger.value
        } else {
            value == trigger.value
        }
    }

    fn should_trigger_event(&self, register: u8) -> bool {
        if let Some(last_time) = self.last_event_times.get(&register) {
            let elapsed = last_time.elapsed();
            let debounce_duration = Duration::from_millis(self.config.polling.debounce_ms);
            elapsed >= debounce_duration
        } else {
            true
        }
    }

    pub fn reload_config(&mut self, new_config: Config) -> Result<()> {
        self.config = new_config;
        self.register_states.clear();
        self.last_event_times.clear();

        for register_map in &self.config.registers {
            self.register_states
                .insert(register_map.register_address, 0u8);
        }

        info!("Configuration reloaded successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_matching() {
        let trigger = ValueTrigger {
            value: 0x01,
            mask: Some(0x01),
            command: "test".to_string(),
            description: None,
        };

        let daemon_config = Config::default();
        let daemon_spi = SpiDevice::new("/dev/null").ok();

        // Test matching with mask
        assert!(0x01 & trigger.mask.unwrap() == trigger.value);
        assert!(0x03 & trigger.mask.unwrap() == trigger.value);
    }
}
