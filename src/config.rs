use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub spi: SpiConfig,
    pub polling: PollingConfig,
    pub buttons: Vec<ButtonMapping>,
    pub klipper: Option<KlipperConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiConfig {
    pub device: String,
    pub speed_hz: u32,
    pub mode: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollingConfig {
    pub interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KlipperConfig {
    /// Base URL for the Klipper API server, e.g. http://127.0.0.1:7125/
    pub base_url: String,
    /// Optional API key or token if the server requires authentication
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonMapping {
    pub button: u8,
    pub config: Option<u8>,
    pub description: Option<String>,
    pub command: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            spi: SpiConfig {
                device: "/dev/spidev0.0".to_string(),
                speed_hz: 1_000_000,
                mode: 0,
            },
            polling: PollingConfig {
                interval_ms: 100,
            },
            buttons: vec![],
            klipper: None,
        }
    }
}
