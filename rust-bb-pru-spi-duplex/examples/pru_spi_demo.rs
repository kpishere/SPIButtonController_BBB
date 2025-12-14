/// Example/Demo for PRU SPI duplex communication
/// This demonstrates using both master and slave in a full-duplex configuration

use anyhow::Result;
use log::{info, error};
use spi_button_controller::pru_context::PRU_DATA_BUFFER_SIZE;
use spi_button_controller::pru_master::PruSpiMaster;
use spi_button_controller::pru_slave::PruSpiSlave;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    // Initialize logging
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .ok();

    info!("PRU SPI Duplex Demo starting...");

    // Create master and slave
    let mut master = PruSpiMaster::new();
    let mut slave = PruSpiSlave::new();

    // Initialize PRUs
    master.init()?;
    slave.init()?;

    // Create a shared flag for graceful shutdown
    let should_stop = Arc::new(AtomicBool::new(false));
    let should_stop_master = Arc::clone(&should_stop);
    let should_stop_slave = Arc::clone(&should_stop);

    // Start both master and slave loops
    master.start::<fn()>(None)?;
    slave.start::<fn()>(None)?;

    // Setup Ctrl+C handler
    let should_stop_ctrl_c = Arc::clone(&should_stop);
    ctrlc::set_handler(move || {
        info!("Received Ctrl+C, stopping...");
        should_stop_ctrl_c.store(true, Ordering::SeqCst);
    })?;

    // Prepare test data
    let length = 0x200 / std::mem::size_of::<i32>();
    let mut original_master_buf = vec![0i32; length];
    let mut original_slave_buf = vec![0i32; length];

    info!("Starting transmission test loop...");
    let mut iteration = 0;

    while !should_stop.load(Ordering::SeqCst) && iteration < 5 {
        iteration += 1;
        info!("Iteration {}", iteration);

        // Prepare data for master (will transmit on MOSI)
        for n in 0..length {
            let value = (n + 1) as i32;
            original_master_buf[n] = value;
        }

        // Prepare data for slave (will transmit on MISO)
        for n in 0..length {
            let value = ((n * 2) + 1) as i32;
            original_slave_buf[n] = value;
        }

        // Copy data to PRU buffers (simulated - in real code would use actual PRU memory)
        info!(
            "Master transmitting {} bytes, Slave receiving {} bytes",
            original_master_buf.len() * std::mem::size_of::<i32>(),
            original_slave_buf.len() * std::mem::size_of::<i32>()
        );

        let transmission_length = (length * std::mem::size_of::<i32>()) as u32;

        // Enable slave receive
        slave.enable_receive(transmission_length);

        // Start master transmission
        master.start_transmission(transmission_length);
        info!("Transmitting {} bytes...", transmission_length);

        // Wait for slave to complete
        slave.wait_for_transmission_to_complete(Duration::from_millis(100));
        info!(
            "Completed: Master sent {} bytes, Slave received {} bytes",
            transmission_length,
            slave.get_last_transmission_length()
        );

        // In a real scenario, we would verify data integrity here
        // For this demo, we just verify that something was transmitted
        let slave_received = slave.get_last_transmission_length();
        if slave_received == transmission_length {
            info!("✓ Transmission length verified");
        } else {
            error!(
                "✗ Transmission length mismatch: expected {}, got {}",
                transmission_length, slave_received
            );
        }

        // Wait before next iteration
        thread::sleep(Duration::from_millis(100));
    }

    info!("Test completed");

    // Cleanup
    master.stop();
    slave.stop();
    master.wait()?;
    slave.wait()?;

    info!("PRU SPI Duplex Demo completed");
    Ok(())
}
