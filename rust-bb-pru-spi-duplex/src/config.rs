use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub spi: SpiConfig,
    pub polling: PollingConfig,
    pub registers: Vec<RegisterMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiConfig {
    pub device: String,
    pub bus: u32,
    pub chip_select: u32,
    pub speed_hz: u32,
    pub mode: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollingConfig {
    pub interval_ms: u64,
    pub debounce_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterMapping {
    pub register_address: u8,
    pub name: String,
    pub value_triggers: Vec<ValueTrigger>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueTrigger {
    pub value: u8,
    pub mask: Option<u8>,
    pub command: String,
    pub description: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            spi: SpiConfig {
                device: "/dev/spidev0.0".to_string(),
                bus: 0,
                chip_select: 0,
                speed_hz: 1_000_000,
                mode: 0,
            },
            polling: PollingConfig {
                interval_ms: 100,
                debounce_ms: 50,
            },
            registers: vec![],
        }
    }
}
