# Rust Translation Implementation Guide

## Project Structure

```
SPIButtonController_BBB/
├── Cargo.toml                    # Project manifest with dependencies
├── src/
│   ├── lib.rs                    # Library root - module exports
│   ├── main.rs                   # Daemon binary entry point
│   ├── pru_context.rs            # Shared memory structure (NEW)
│   ├── pru_master.rs             # PRU 0 master controller (NEW)
│   ├── pru_slave.rs              # PRU 1 slave controller (NEW)
│   ├── ffi.rs                    # FFI bindings documentation (NEW)
│   ├── config.rs                 # Configuration structures
│   ├── command.rs                # Command execution
│   ├── daemon.rs                 # Main daemon loop
│   └── spi.rs                    # Generic SPI device
├── examples/
│   ├── config.yaml               # Example configuration
│   └── pru_spi_demo.rs           # Full-duplex demo (NEW)
├── RUST_TRANSLATION.md           # Comprehensive translation docs (NEW)
└── systemd/
    └── spi-button-controller.service
```

## Key Translation Patterns

### 1. Class to Struct + impl

**C++ Original:**
```cpp
class PruSpiMaster {
private:
    bool _pruInited;
    RT_TASK _loopTask;
public:
    int init();
    void cleanup();
};
```

**Rust Translation:**
```rust
pub struct PruSpiMaster {
    pru_inited: bool,
    loop_thread: Option<std::thread::JoinHandle<()>>,
}

impl PruSpiMaster {
    pub fn init(&mut self) -> Result<()> { ... }
    pub fn cleanup(&mut self) { ... }
}
```

**Key Differences:**
- Return `Result<()>` instead of `int` error codes
- Mutable self for stateful operations
- Option types for thread handles
- Explicit error handling

### 2. Pointers to Shared Memory

**C++ Original:**
```cpp
uint8_t* _pruMem;
volatile int* _externalShouldStop;
```

**Rust Translation:**
```rust
pru_mem: Option<Arc<AtomicPtr<u8>>>,
external_should_stop: Arc<AtomicBool>,
```

**Advantages:**
- Arc handles reference counting automatically
- AtomicPtr/AtomicBool for thread-safe access
- Option prevents null pointer dereferences
- Compile-time borrow checking

### 3. Real-time Task to Thread

**C++ Original:**
```cpp
rt_task_create(&_loopTask, ...);
rt_task_start(&_loopTask, loop, this);
rt_task_sleep(300000);
```

**Rust Translation:**
```rust
let thread_handle = thread::spawn(move || {
    Self::loop_fn(context, should_stop, ...);
});
this.loop_thread = Some(thread_handle);

thread::sleep(Duration::from_micros(300000));
```

**Notes:**
- std::thread for basic threading
- Move closures capture ownership
- Arc for shared references
- Duration API is more type-safe

### 4. Callback Function Pointers

**C++ Original:**
```cpp
void(*_callback)(void*);
_callback(arg);
```

**Rust Translation:**
```rust
callback: Arc<parking_lot::Mutex<Option<Box<dyn Fn() + Send + 'static>>>>,
if let Some(cb) = callback.lock().as_ref() {
    cb();
}
```

**Design:**
- Trait objects for flexibility
- Boxing for heap allocation
- Mutex for thread-safe modification
- Send + 'static bounds for thread safety

### 5. Volatile Access

**C++ Original:**
```cpp
while(!shouldStop() || *_externalShouldStop) { ... }
```

**Rust Translation:**
```rust
while !self.should_stop.load(Ordering::SeqCst) || 
      !self.external_should_stop.load(Ordering::SeqCst) { ... }
```

**Atomic Operations:**
- `Ordering::SeqCst`: Strongest guarantee, most overhead
- `Ordering::Release`: For writes signaling completion
- `Ordering::Acquire`: For reads checking signals
- `Ordering::Relaxed`: When no synchronization needed

### 6. PRU Memory Mapping

**C++ Original:**
```cpp
prussdrv_map_prumem(
    PRU_SPI_MASTER_NUM == 0 ? PRUSS0_PRU0_DATARAM : PRUSS0_PRU1_DATARAM,
    (void **)&_pruMem
);
context = (PruSpiContext*) _pruMem;
```

**Rust Translation (Placeholder):**
```rust
unsafe {
    let ctx_ptr = self.context.load(Ordering::SeqCst);
    if !ctx_ptr.is_null() {
        let ctx = &mut *ctx_ptr;
        ctx.length = transmission_length;
    }
}
```

**Safety:**
- Unsafe block documents pointer manipulation
- Null checks prevent segfaults
- RAII handles cleanup automatically

## Testing Strategy

### Unit Tests

Each module includes tests for:

**pru_context.rs:**
- Memory layout validation
- Buffer access patterns
- Reset functionality

**pru_master.rs:**
- Master initialization state
- Should-stop signal handling
- Transmission state tracking

**pru_slave.rs:**
- Slave initialization state
- Reception enable functionality
- Transmission length tracking

### Integration Tests

The `examples/pru_spi_demo.rs` demonstrates:
1. Master and slave initialization
2. Data preparation
3. Full-duplex transmission
4. Graceful shutdown with Ctrl+C
5. Data integrity verification (placeholder)

Run integration test:
```bash
cargo run --example pru_spi_demo
```

## Memory Safety Analysis

### Eliminated Vulnerabilities

1. **Buffer Overflow**: Fixed-size arrays with bounds checking
2. **Use-After-Free**: RAII automatic cleanup via Drop trait
3. **Data Races**: Atomic operations and Arc synchronization
4. **Memory Leaks**: Reference counting and ownership system
5. **Null Pointers**: Option type forces null handling

### Remaining Unsafe Blocks

Unsafe code is minimal and well-documented:
- Pointer dereference for PRU memory (necessary)
- Atomic loads/stores (required for hardware communication)
- All unsafe operations have SAFETY comments

## FFI Integration Steps

### Step 1: Create FFI Bindings

Create `src/ffi_impl.rs`:
```rust
use libc::{c_int, c_uint, c_void};

extern "C" {
    pub fn prussdrv_init() -> c_int;
    pub fn prussdrv_exit() -> c_int;
    // ... more functions
}
```

### Step 2: Wrap FFI Functions

Create safe Rust wrappers:
```rust
pub fn init_pru() -> Result<()> {
    unsafe {
        let ret = prussdrv_init();
        if ret == 0 {
            Ok(())
        } else {
            Err(anyhow!("prussdrv_init failed: {}", ret))
        }
    }
}
```

### Step 3: Update Library Linking

Add to `Cargo.toml`:
```toml
[dependencies]
libc = "0.2"

[build]
rustflags = ["-l", "prussdrv"]
```

Or use `build.rs`:
```rust
fn main() {
    println!("cargo:rustflags=-l prussdrv");
}
```

## Performance Optimization

### Current Implementation

- **Single-threaded PRU monitoring**: 300µs sleep interval
- **Double buffering**: Prevents tearing during transfers
- **Atomic operations**: Lock-free synchronization
- **No allocations in loop**: Constant memory usage

### Potential Improvements

1. **Tune sleep interval**: Profile on real hardware
2. **Add performance metrics**: Track transfer rates
3. **Optimize buffer size**: Profile memory usage
4. **Reduce atomic operations**: Batch updates
5. **Event-based instead of polling**: Use interrupts when available

## Error Handling Patterns

### Result Type Usage

All fallible operations return `Result<T, E>`:
```rust
pub fn init(&mut self) -> Result<()> {
    // Returns Ok(()) on success or Err(e) on failure
}
```

### Error Propagation

Use `?` operator for clean error propagation:
```rust
pub fn demo() -> Result<()> {
    let mut master = PruSpiMaster::new();
    master.init()?;  // Return early if init fails
    master.start()?;
    Ok(())
}
```

### Error Context

Add context to errors for debugging:
```rust
File::open(path).context(format!("Failed to open {}", path))?;
```

## Logging Strategy

### Log Levels Used

- **ERROR**: Initialization failures, critical errors
- **WARN**: Non-critical issues, potential problems
- **INFO**: State transitions, major operations
- **DEBUG**: Detailed operation information
- **TRACE**: Very detailed debugging (not used currently)

### Enabling Logging

Development:
```bash
RUST_LOG=debug cargo run
```

Production:
```bash
export RUST_LOG=info
spi-button-controller /etc/config.yaml
```

## Documentation Standards

Each public module includes:
1. Module-level documentation comment
2. Type documentation with usage examples
3. Method documentation with error conditions
4. Unit tests demonstrating functionality

Example:
```rust
/// PRU SPI Context - shared memory structure between ARM and PRU cores
///
/// This structure maintains the state of SPI communication,
/// including buffers, transmission length, and current buffer index.
///
/// # Examples
///
/// ```rust
/// let mut ctx = PruSpiContext::new();
/// ctx.get_buffer_mut()[0] = 0x42;
/// ```
#[repr(C)]
#[derive(Debug)]
pub struct PruSpiContext {
    // ...
}
```

## Building for Release

### Optimization Configuration

```toml
[profile.release]
opt-level = 3        # Maximum optimization
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit for optimization
strip = true         # Strip debug symbols
```

Build release binary:
```bash
cargo build --release
```

Result: `target/release/spi-button-controller` (~4-6 MB)

## Cross-Compilation for BBB

### Target Configuration

Add `.cargo/config.toml`:
```toml
[build]
target = "arm-unknown-linux-gnueabihf"

[target.arm-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
```

Install ARM toolchain:
```bash
rustup target add arm-unknown-linux-gnueabihf
sudo apt install arm-linux-gnueabihf-gcc
```

Build for BBB:
```bash
cargo build --release --target arm-unknown-linux-gnueabihf
```

Copy to BBB:
```bash
scp target/arm-unknown-linux-gnueabihf/release/spi-button-controller \
    root@beaglebone:/usr/local/bin/
```

## Troubleshooting

### Compilation Errors

**Error: "could not find prussdrv bindings"**
- Solution: FFI bindings are currently placeholders (see ffi.rs)
- Create proper bindings or use a C library wrapper

**Error: "incompatible architectures"**
- Solution: Ensure proper cross-compilation setup
- Verify rustup target installation

### Runtime Errors

**Error: "SPI device not found"**
- Solution: Ensure device tree overlay is loaded
- Check `/dev/spidevX.X` exists

**Error: "Failed to map PRU memory"**
- Solution: Ensure PRU firmware is loaded
- Check `/proc/device-tree/` for PRU configuration

**Error: "Transmission timeout"**
- Solution: Verify PRU firmware is running
- Check pin configuration
- Enable logging to debug

## Next Steps for Production

1. **Add FFI bindings**: Create full prussdrv-sys wrapper
2. **Real-time kernel**: Consider PREEMPT_RT for deterministic behavior
3. **Error recovery**: Add timeout-based recovery mechanisms
4. **Performance monitoring**: Add metrics collection
5. **Extended testing**: Hardware validation on real BBB
6. **Documentation**: Device tree overlay creation
7. **Packaging**: Create systemd service and configuration files

## References

- [Rust FFI Guide](https://doc.rust-lang.org/nomicon/ffi.html)
- [BeagleBone Black PRU](https://beagleboard.org/pru)
- [PRU Reference](https://ti.com/tool/pru-training)
- [Tokio async runtime](https://tokio.rs/)
- [Serde serialization](https://serde.rs/)
