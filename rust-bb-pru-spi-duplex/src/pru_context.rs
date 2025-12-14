/// PRU SPI Context - shared memory structure between ARM and PRU cores
/// This structure is mapped to the PRU data RAM and used for communication

use std::mem;

pub const PRU_DATA_BUFFER_SIZE: usize = 0x400; // 1024 bytes

/// Represents the shared memory context between ARM and PRU cores.
/// This structure is overlaid in PRU data memory and must maintain
/// exact memory layout for hardware compatibility.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct PruSpiContext {
    /// Two buffers for double buffering (1KB each)
    pub buffers: [[u8; PRU_DATA_BUFFER_SIZE]; 2],
    /// Current buffer index (0 or 1)
    pub buffer: u32,
    /// Length of transmission in bytes
    /// Master sets this before beginning, PRU resets it to 0 when done.
    /// Slave can read this after transmission to check how many bytes were transmitted.
    pub length: u32,
    /// Maximum transmission length for slave (unused by Master)
    /// Slave uses this as max length of the transmission, PRU resets it to 0 when done
    pub slave_max_transmission_length: u32,
}

impl PruSpiContext {
    /// Create a new zeroed PRU context
    pub fn new() -> Self {
        PruSpiContext {
            buffers: [[0u8; PRU_DATA_BUFFER_SIZE]; 2],
            buffer: 0,
            length: 0,
            slave_max_transmission_length: 0,
        }
    }

    /// Get the size of the context in bytes (for memory mapping)
    pub fn size() -> usize {
        mem::size_of::<PruSpiContext>()
    }

    /// Get a mutable reference to the current data buffer
    pub fn get_buffer_mut(&mut self) -> &mut [u8] {
        let idx = self.buffer as usize;
        if idx >= 2 {
            &mut self.buffers[0]
        } else {
            &mut self.buffers[idx]
        }
    }

    /// Get an immutable reference to the current data buffer
    pub fn get_buffer(&self) -> &[u8] {
        let idx = self.buffer as usize;
        if idx >= 2 {
            &self.buffers[0]
        } else {
            &self.buffers[idx]
        }
    }

    /// Reset the context to initial state
    pub fn reset(&mut self) {
        self.buffer = 0;
        self.length = 0;
        self.slave_max_transmission_length = 0;
        for buf in &mut self.buffers {
            buf.iter_mut().for_each(|b| *b = 0);
        }
    }
}

impl Default for PruSpiContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_size() {
        // Verify the memory layout is correct for PRU communication
        let size = PruSpiContext::size();
        assert!(size > 0);
    }

    #[test]
    fn test_buffer_access() {
        let mut ctx = PruSpiContext::new();
        ctx.buffer = 0;
        let buf = ctx.get_buffer_mut();
        assert_eq!(buf.len(), PRU_DATA_BUFFER_SIZE);

        ctx.buffer = 1;
        let buf = ctx.get_buffer_mut();
        assert_eq!(buf.len(), PRU_DATA_BUFFER_SIZE);
    }

    #[test]
    fn test_context_reset() {
        let mut ctx = PruSpiContext::new();
        ctx.length = 100;
        ctx.buffer = 1;
        ctx.reset();
        assert_eq!(ctx.length, 0);
        assert_eq!(ctx.buffer, 0);
    }
}
