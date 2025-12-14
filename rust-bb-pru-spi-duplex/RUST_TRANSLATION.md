# SPI Button Controller - Rust Implementation

A complete Rust translation of the BeagleBone Black (BBB) PRU SPI duplex communication project (`bb-pru-spi-duplex`).

## Overview

This project provides a Rust-based implementation for full-duplex SPI communication on BeagleBone Black using the Programmable Real-time Unit (PRU) cores. It translates the original C++ implementation to idiomatic Rust, maintaining functionality while providing memory safety and better error handling.

## Original Project

- **Source**: https://github.com/giuliomoro/bb-pru-spi-duplex.git
- **Purpose**: Full-duplex SPI master/slave communication using PRU 0 and PRU 1 on BeagleBone Black

## Architecture

### Key Components

1. **PruSpiContext** (`src/pru_context.rs`)
   - Shared memory structure for ARM ↔ PRU communication
   - Manages two 1KB buffers for double-buffered operation
   - Tracks transmission state and buffer indices

2. **PruSpiMaster** (`src/pru_master.rs`)
   - Controls PRU 0 as SPI master
   - Manages transmission initiation and completion
   - Handles real-time loop for buffer monitoring
   - Supports optional callbacks on buffer changes

3. **PruSpiSlave** (`src/pru_slave.rs`)
   - Controls PRU 1 as SPI slave
   - Manages reception with maximum transmission length
   - Monitors transmission status
   - Supports optional callbacks on buffer changes

4. **SPI Device** (`src/spi.rs`)
   - Basic SPI device communication wrapper
   - Read/write register operations
   - Device path management

5. **Configuration** (`src/config.rs`)
   - YAML configuration support
   - SPI parameters (device, speed, mode)
   - Register mapping and triggers
   - Polling and debounce settings

6. **Command Execution** (`src/command.rs`)
   - Execute system commands in response to SPI events
   - Support for timeouts
   - Comprehensive error handling

7. **Daemon** (`src/daemon.rs`)
   - Main event loop for monitoring SPI registers
   - Trigger-based command execution
   - Configuration reload support

## Translation Notes

### From C++ to Rust

| C++ Feature | Rust Equivalent | Notes |
|------------|-----------------|-------|
| Class members | Struct fields | Direct translation |
| Class methods | impl blocks | Rust idioms applied |
| Pointers to PRU memory | Arc<AtomicPtr<T>> | Thread-safe memory sharing |
| RT_TASK | std::thread | Simpler threading model |
| Callbacks | Fn() trait objects | Boxed for flexibility |
| Memory alignment | #[repr(C)] | Maintains hardware compatibility |
| Global state | Arc<AtomicBool> | Atomic for thread safety |
| INTC mapping | Constants | Placeholder for FFI |

### Key Improvements

1. **Memory Safety**: Eliminates buffer overflows and use-after-free bugs
2. **Error Handling**: Result types for better error propagation
3. **Thread Safety**: Atomic operations prevent race conditions
4. **Resource Management**: RAII pattern ensures cleanup
5. **Type Safety**: Stronger type checking at compile time

## Data Flow

### Full-Duplex Operation

```
Master Side:        Slave Side:

prepare_data()      prepare_data()
    ↓                   ↓
set_length()        enable_receive()
    ↓                   ↓
start_transmission()    ↓
    ↓                   ↓
   [PRU SPI Transfer]
    ↓                   ↓
wait_transmission()  wait_transmission()
    ↓                   ↓
read_response()     read_transmission_length()
```

## Building and Testing

### Build
```bash
cargo build --release
```

### Check
```bash
cargo check
```

### Run Demo
```bash
cargo run --example pru_spi_demo
```

### Run Tests
```bash
cargo test
```

### Full Documentation
```bash
cargo doc --open
```

## PRU Firmware Requirements

The following binary firmware files must be present:
- `/root/spi-duplex/pru-spi-master.bin` - PRU 0 master firmware
- `/root/spi-duplex/pru-spi-slave.bin` - PRU 1 slave firmware

These are compiled from the `.p` files in the original project:
- `pru-spi-master.p` - Master firmware source
- `pru-spi-slave.p` - Slave firmware source

## Pin Configuration

### Master (PRU 0)
- **CS** (Chip Select): P9_27 (R30.5)
- **MOSI** (Master Out): P8_11 (R30.15)
- **MISO** (Master In): P8_15 (R31.15)
- **SCK** (Clock): P8_12 (R30.14)

### Slave (PRU 1)
- **CS** (Chip Select): P8_44 (R31.3)
- **MOSI** (Master In): P8_45 (R31.0)
- **MISO** (Master Out): P8_43 (R30.2)
- **SCK** (Clock): P8_46 (R31.1)

## Usage Examples

### Basic Master Transmission

```rust
use spi_button_controller::pru_master::PruSpiMaster;

let mut master = PruSpiMaster::new();
master.init()?;
master.start::<fn()>(None)?;

// Prepare data
let data = master.get_data_mut().unwrap();
data[0] = 0xA5;
data[1] = 0x5A;

// Transmit 2 bytes
master.start_transmission(2);
master.wait_for_transmission_to_complete(Duration::from_millis(100));

master.stop();
master.wait()?;
```

### Slave with Receive

```rust
use spi_button_controller::pru_slave::PruSpiSlave;

let mut slave = PruSpiSlave::new();
slave.init()?;
slave.start::<fn()>(None)?;

// Enable receive for 256 bytes
slave.enable_receive(256);
slave.wait_for_transmission_to_complete(Duration::from_millis(100));

let received_length = slave.get_last_transmission_length();
println!("Received {} bytes", received_length);

slave.stop();
slave.wait()?;
```

### Full-Duplex Demo

See `examples/pru_spi_demo.rs` for a complete working example with both master and slave.

## Dependencies

Key Rust crates used:
- **tokio**: Async runtime for daemon operations
- **serde/serde_yaml**: Configuration serialization
- **log/env_logger**: Structured logging
- **parking_lot**: Efficient mutex implementation
- **ctrlc**: Graceful shutdown handling
- **anyhow/thiserror**: Error handling

## Testing

The project includes unit tests for:
- Context memory layout validation
- Buffer access patterns
- Stop signal handling
- Transmission completion detection

Run tests:
```bash
cargo test
```

## Systemd Integration

The original project includes a systemd service file. A Rust version can be installed:

```bash
sudo cp systemd/spi-button-controller.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable spi-button-controller
sudo systemctl start spi-button-controller
```

## Configuration

YAML-based configuration (see `examples/config.yaml`):

```yaml
spi:
  device: /dev/spidev0.0
  speed_hz: 1000000
  mode: 0

polling:
  interval_ms: 100
  debounce_ms: 50

registers:
  - register_address: 0x01
    name: "Button Status"
    value_triggers:
      - value: 0x01
        command: "systemctl reboot"
        description: "Reboot on button press"
```

## Performance Considerations

1. **PRU vs ARM**: PRU operations are real-time, not preempted
2. **Buffer Size**: 1KB per buffer (2KB total) - configurable in pru_context.rs
3. **Loop Sleep**: 300µs default sleep to prevent busy-waiting
4. **Thread Priority**: Placeholder for real-time task priority (would need PREEMPT_RT kernel)

## Limitations and Future Work

1. **FFI Bindings**: Currently placeholders - needs actual prussdrv-sys bindings
2. **Real-time**: Requires PREEMPT_RT kernel for true real-time behavior
3. **Error Recovery**: Could add timeout-based error handling
4. **Statistics**: Could add performance metrics collection
5. **Advanced Callbacks**: Could support multiple callbacks per event

## FFI Integration (Future)

To integrate with actual prussdrv library:

```rust
// Add to Cargo.toml
[dependencies]
prussdrv-sys = { path = "../prussdrv-sys" }

// Use in pru_master.rs
unsafe {
    prussdrv_init()?;
    prussdrv_open(PRU_EVTOUT_0)?;
    // ... etc
}
```

## Debugging

Enable verbose logging:
```bash
RUST_LOG=debug cargo run --release
```

Or in systemd:
```bash
sudo systemctl set-environment RUST_LOG=debug
```

## License

Original C++ project: Check the original repository
Rust translation: Available under the same license as original

## References

- [BeagleBone Black PRU Documentation](https://beagleboard.org/pru)
- [PRU Training](https://ti.com/tool/pru-training)
- [Rust FFI Guide](https://doc.rust-lang.org/nomicon/ffi.html)
