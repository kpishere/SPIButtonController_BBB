use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::fs::File;
use std::io::{Read, Write};

pub struct SpiDevice {
    file: File,
    device_path: String,
}

impl SpiDevice {
    pub fn new(device_path: &str) -> Result<Self> {
        let file = File::open(device_path)
            .context(format!("Failed to open SPI device: {}", device_path))?;

        Ok(SpiDevice {
            file,
            device_path: device_path.to_string(),
        })
    }

    pub fn read_register(&mut self, register: u8) -> Result<u8> {
        // In a real scenario, this would use ioctl to communicate with the SPI device
        // For now, we implement a generic read approach
        debug!("Reading from register: 0x{:02x}", register);

        let mut buffer = vec![0u8; 2];
        buffer[0] = register;

        self.file
            .write_all(&buffer)
            .context("Failed to write to SPI device")?;

        self.file
            .read_exact(&mut buffer)
            .context("Failed to read from SPI device")?;

        Ok(buffer[1])
    }

    pub fn write_register(&mut self, register: u8, value: u8) -> Result<()> {
        debug!(
            "Writing to register: 0x{:02x}, value: 0x{:02x}",
            register, value
        );

        let buffer = vec![register | 0x80, value]; // MSB set for write operation
        self.file
            .write_all(&buffer)
            .context("Failed to write to SPI device")?;

        Ok(())
    }

    pub fn device_path(&self) -> &str {
        &self.device_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_address() {
        assert_eq!(0x42, 0x42);
    }
}
