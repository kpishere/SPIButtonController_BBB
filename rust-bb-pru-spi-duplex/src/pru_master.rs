/// PRU SPI Master implementation
/// Controls the SPI master operation on PRU 0

use crate::pru_context::PruSpiContext;
use anyhow::{anyhow, Result};
use log::{debug, info};
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const PRU_SPI_MASTER_NUM: u32 = 0;
#[allow(dead_code)]
const PRU_SPI_MASTER_NUM_CONST: u32 = PRU_SPI_MASTER_NUM;

/// PRU SPI Master controller
pub struct PruSpiMaster {
    pru_inited: bool,
    pru_enabled: bool,
    pru_mem: Option<Arc<AtomicPtr<u8>>>,
    context: Arc<AtomicPtr<PruSpiContext>>,
    should_stop: Arc<AtomicBool>,
    external_should_stop: Arc<AtomicBool>,
    loop_thread: Option<std::thread::JoinHandle<()>>,
    callback: Arc<parking_lot::Mutex<Option<Box<dyn Fn() + Send + 'static>>>>,
}

impl PruSpiMaster {
    /// Create a new PRU SPI Master instance
    pub fn new() -> Self {
        PruSpiMaster {
            pru_inited: false,
            pru_enabled: false,
            pru_mem: None,
            context: Arc::new(AtomicPtr::new(std::ptr::null_mut())),
            should_stop: Arc::new(AtomicBool::new(false)),
            external_should_stop: Arc::new(AtomicBool::new(false)),
            loop_thread: None,
            callback: Arc::new(parking_lot::Mutex::new(None)),
        }
    }

    /// Initialize the PRU
    pub fn init(&mut self) -> Result<()> {
        info!("Initializing PRU SPI Master...");

        if !self.pru_inited {
            // Initialize prussdrv - would normally call prussdrv_init() via FFI
            // For now, this is a placeholder that shows the structure
            debug!("Calling prussdrv_init()");
            
            // In a real implementation with prussdrv bindings:
            // unsafe { prussdrv_init() }.context("prussdrv_init failed")?;
            
            self.pru_inited = true;
        }

        // Open PRU Interrupt
        debug!("Opening PRU interrupt");
        // In real implementation: prussdrv_open(PRU_EVTOUT_0)

        // Map PRU memory
        debug!("Mapping PRU memory");
        // In real implementation:
        // let pru_mem = prussdrv_map_prumem(
        //     if PRU_SPI_MASTER_NUM == 0 { PRUSS0_PRU0_DATARAM } else { PRUSS0_PRU1_DATARAM }
        // )

        if !self.pru_enabled {
            debug!("Enabling PRU program");
            // In real implementation:
            // prussdrv_exec_program(PRU_SPI_MASTER_NUM, "/root/spi-duplex/pru-spi-master.bin")
            self.pru_enabled = true;
        }

        info!("PRU SPI Master initialized successfully");
        Ok(())
    }

    /// Start the PRU loop with optional callback
    pub fn start<F>(&mut self, callback: Option<F>) -> Result<()>
    where
        F: Fn() + Send + 'static,
    {
        info!("Starting PRU SPI Master loop");

        if let Some(cb) = callback {
            *self.callback.lock() = Some(Box::new(cb));
        }

        self.should_stop.store(false, Ordering::SeqCst);

        let context = Arc::clone(&self.context);
        let should_stop = Arc::clone(&self.should_stop);
        let external_should_stop = Arc::clone(&self.external_should_stop);
        let callback = Arc::clone(&self.callback);

        let thread_handle = thread::spawn(move || {
            Self::loop_fn(context, should_stop, external_should_stop, callback);
        });

        self.loop_thread = Some(thread_handle);
        info!("PRU SPI Master loop started");
        Ok(())
    }

    /// Stop the PRU loop
    pub fn stop(&mut self) {
        info!("Stopping PRU SPI Master");
        self.should_stop.store(true, Ordering::SeqCst);
        *self.callback.lock() = None;
    }

    /// Wait for the loop to finish
    pub fn wait(&mut self) -> Result<()> {
        if let Some(thread) = self.loop_thread.take() {
            thread.join().map_err(|_| anyhow!("Failed to join loop thread"))?;
        }
        Ok(())
    }

    /// Check if transmission is complete
    pub fn is_transmission_done(&self) -> bool {
        unsafe {
            let ctx_ptr = self.context.load(Ordering::SeqCst);
            if !ctx_ptr.is_null() {
                (*ctx_ptr).length == 0
            } else {
                false
            }
        }
    }

    /// Wait for transmission to complete
    pub fn wait_for_transmission_to_complete(&self, sleep_time: Duration) {
        while !self.should_stop.load(Ordering::SeqCst)
            && !self.external_should_stop.load(Ordering::SeqCst)
            && !self.is_transmission_done()
        {
            thread::sleep(sleep_time);
        }
    }

    /// Get the current buffer index
    pub fn get_buffer(&self) -> u32 {
        unsafe {
            let ctx_ptr = self.context.load(Ordering::SeqCst);
            if !ctx_ptr.is_null() {
                (*ctx_ptr).buffer
            } else {
                0
            }
        }
    }

    /// Start a transmission with specified length
    pub fn start_transmission(&self, length: u32) {
        unsafe {
            let ctx_ptr = self.context.load(Ordering::SeqCst);
            if !ctx_ptr.is_null() {
                (*ctx_ptr).length = length;
            }
        }
    }

    /// Get mutable reference to data buffer for writing
    pub fn get_data_mut(&self) -> Option<&mut [u8]> {
        unsafe {
            let ctx_ptr = self.context.load(Ordering::SeqCst);
            if !ctx_ptr.is_null() {
                let ctx = &mut *ctx_ptr;
                Some(&mut ctx.buffers[ctx.buffer as usize])
            } else {
                None
            }
        }
    }

    /// Get immutable reference to data buffer for reading
    pub fn get_data(&self) -> Option<&[u8]> {
        unsafe {
            let ctx_ptr = self.context.load(Ordering::SeqCst);
            if !ctx_ptr.is_null() {
                let ctx = &*ctx_ptr;
                Some(&ctx.buffers[ctx.buffer as usize])
            } else {
                None
            }
        }
    }

    /// Cleanup resources
    pub fn cleanup(&mut self) {
        if self.pru_enabled {
            debug!("Disabling PRU");
            // prussdrv_pru_disable(PRU_SPI_MASTER_NUM);
            self.pru_enabled = false;
        }

        if self.pru_inited {
            debug!("Exiting PRU driver");
            // prussdrv_exit();
            self.pru_inited = false;
        }

        self.pru_mem = None;
        *self.callback.lock() = None;
    }

    /// Internal loop function that monitors buffer changes
    fn loop_fn(
        context: Arc<AtomicPtr<PruSpiContext>>,
        should_stop: Arc<AtomicBool>,
        external_should_stop: Arc<AtomicBool>,
        callback: Arc<parking_lot::Mutex<Option<Box<dyn Fn() + Send + 'static>>>>,
    ) {
        let mut last_buffer: u32 = 0;

        loop {
            if should_stop.load(Ordering::SeqCst) || external_should_stop.load(Ordering::SeqCst) {
                break;
            }

            unsafe {
                let ctx_ptr = context.load(Ordering::SeqCst);
                if !ctx_ptr.is_null() {
                    let current_buffer = (*ctx_ptr).buffer;
                    if last_buffer == current_buffer {
                        thread::sleep(Duration::from_micros(300000));
                        continue;
                    }

                    last_buffer = current_buffer;

                    if let Some(cb) = callback.lock().as_ref() {
                        cb();
                    }
                }
            }
        }
    }

    /// Check if should stop
    pub fn should_stop(&self) -> bool {
        self.should_stop.load(Ordering::SeqCst) || self.external_should_stop.load(Ordering::SeqCst)
    }
}

impl Default for PruSpiMaster {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PruSpiMaster {
    fn drop(&mut self) {
        self.stop();
        let _ = self.wait();
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_creation() {
        let master = PruSpiMaster::new();
        assert!(!master.pru_inited);
        assert!(!master.pru_enabled);
    }

    #[test]
    fn test_should_stop() {
        let mut master = PruSpiMaster::new();
        assert!(!master.should_stop());
        master.should_stop.store(true, Ordering::SeqCst);
        assert!(master.should_stop());
    }
}
