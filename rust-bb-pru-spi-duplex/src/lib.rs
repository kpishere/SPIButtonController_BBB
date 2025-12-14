/// SPI Button Controller - Rust implementation for BeagleBone Black with PRU
///
/// This library provides functionality for PRU-based SPI communication on BeagleBone Black,
/// including master and slave implementations for full-duplex SPI communication.

pub mod command;
pub mod config;
pub mod daemon;
pub mod ffi;
pub mod pru_context;
pub mod pru_master;
pub mod pru_slave;
pub mod spi;

// Re-export main types for convenience
pub use pru_context::PruSpiContext;
pub use pru_master::PruSpiMaster;
pub use pru_slave::PruSpiSlave;
