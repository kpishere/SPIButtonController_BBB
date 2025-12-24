use crate::command::{CommandExecutor, EventMessage};
use crate::config::{Config, ButtonMapping};
use spibuttonlib::{SPIButtonController, SPIButtonState, SPIButton};
use anyhow::Result;
use log::{info, warn};
use std::time::{Duration};
use tokio::time::sleep;

pub struct Daemon {
    spi: SPIButtonController,
    config: Config,
    response_tx: Option<tokio::sync::mpsc::Sender<EventMessage>>,
    id_next: u32,
}

impl Daemon {
    pub fn new(config: Config, response_tx: Option<tokio::sync::mpsc::Sender<EventMessage>>) -> Result<Self> {
        let spi_res = SPIButtonController::new(config.buttons.len(), &config.spi.device, config.spi.speed_hz, config.spi.mode);
        match spi_res {
            Ok(mut spi) => {
                info!("SPI device initialized: {}", config.spi.device);
                info!("Polling interval: {}ms", config.polling.interval_ms);
                info!("Monitoring {} buttons(s)", config.buttons.len());
        
                Daemon::init(&config, &mut spi);

                Ok(Daemon {
                    spi,
                    config,
                    response_tx,
                    id_next: 0,
                })        
            }
            Err(e) => {
                println!("error: {}", e);
                panic!("SPI initialization error.")
            }
        }
    }

    fn init(config: &Config, spi: &mut SPIButtonController)
    {
        for register_map in &config.buttons {
            let btn = SPIButton::new( register_map.config.unwrap_or( SPIButtonState::OnChange as u8 ) );
            spi.set_button(register_map.button, btn);
            info!(
                "  - Button {:?}: {:?}",
                register_map.button, register_map.description
            );
        }
    }

    pub async fn poll(&mut self) -> Result<()> {
        let events = self.spi.loop_once().expect("Controller poll error.");

        // The application logic
        for i in 0..events.len() {
            let mut b = events[i];
            println!("Button {}: State {:?}", b.id(), b.get_state());
            /*
            if b.is_hold_event() {
                match b.get_state() {
                    SPIButtonState::Off => b.set_state(SPIButtonState::On),
                    SPIButtonState::On => b.set_state(SPIButtonState::Flash1),
                    SPIButtonState::Flash1 => b.set_state(SPIButtonState::Flash2),
                    SPIButtonState::Flash2 => b.set_state(SPIButtonState::Off),
                    _ => {}
                }
                b.clear_hold_event();
                controller.set_button(b.id(), b);
            }
            */
            match b.get_state() {
                SPIButtonState::On => {
                    // Process value triggers
                    self.process_triggers(&mut b)
                        .await;
                    self.spi.set_button(b.id(), b);
                },
                _ => {}
            }
        }



        // Sleep for the configured polling interval
        sleep(Duration::from_millis(self.config.polling.interval_ms)).await;

        Ok(())
    }

    async fn process_triggers(
        &mut self,
        button: &mut SPIButton,
    ) {        
        // Execute the associated command
        let cfg_button: &ButtonMapping = &self.config.buttons[button.id() as usize];
        let cmd = cfg_button.command.trim();

        if cmd.starts_with("klipper:") {
            // Klipper API command syntax: klipper:METHOD|<JSON_PARAMS>
            if let Some(klipper_cfg) = &self.config.klipper {
                if let Some(tx) = &self.response_tx {
                    let cmd_clone = cmd.to_string();
                    let klipper_clone = klipper_cfg.clone();
                    let tx_clone = tx.clone();

                    // Generate request id and notify main loop that a request was issued
                    self.id_next += 1;
                    let request_id = self.id_next;
                    let trigger_info = format!("button_id={} desc={:?}", button.id(), cfg_button.description);
                    // send Issued event so main can persist metadata
                    let _ = tx.clone().try_send(EventMessage::Issued { request_id: request_id.clone(), trigger_info: trigger_info.clone() });

                    // spawn the async request using the supplied request_id
                    tokio::spawn(async move {
                        CommandExecutor::send_klipper_command(&cmd_clone, &klipper_clone, request_id, tx_clone).await;
                    });

                    info!("Dispatched Klipper command from button {:?}", cfg_button.description);
                } else {
                    warn!("Klipper command requested but no response queue configured");
                    button.set_state(SPIButtonState::Flash2);
                }
            } else {
                warn!("Klipper command requested but no klipper config provided");
                button.set_state(SPIButtonState::Flash2);
            }
        } else {
            match CommandExecutor::execute(&cfg_button.command) {
                Ok(_) => {
                    info!(
                        "Successfully executed command for trigger on register {:?}",
                        cfg_button.description
                    );
                    button.set_state(SPIButtonState::Off);
                }
                Err(e) => {
                    warn!(
                        "Failed to execute command for register {:?}: {}",
                        cfg_button.description, e
                    );
                    button.set_state(SPIButtonState::Flash2);
                }
            }
        }
    }

    pub fn reload_config(&mut self, new_config: Config) -> Result<()> {
        self.config = new_config;
        Daemon::init(&self.config, &mut self.spi);
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
