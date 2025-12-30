# SPI Button Controller Daemon

A Linux daemon service written in Rust that monitors an SPI device for register value changes and executes arbitrary shell commands based on those values.

## Features

- **Async/await based polling** - Non-blocking SPI device monitoring using Tokio
- **Configuration file driven** - YAML configuration for registers and command mappings
- **Graceful shutdown** - Handles SIGTERM and SIGINT signals properly
- **Configuration reload** - Send SIGHUP to reload configuration without restarting
- **Systemd integration** - Runs as a native Linux daemon with journald logging
- **Shell command execution** - Execute arbitrary shell commands on register value changes
- **Send commands to Klipper API** - Send command and handle response

## Requirements

- Rust 1.70+ (for building)
- Linux kernel with SPI support
- Access to SPI device (typically `/dev/spidev*`)

## Building

```bash
cargo build --release
```

The compiled binary will be at `target/release/spi-button-controller`.

## Configuration

Configuration is defined in YAML format. See `examples/config.yaml` for a complete example.

### Configuration Structure

```yaml
spi:
  device: /dev/spidev0.0          # SPI device path
  speed_hz: 1000000               # SPI clock speed in Hz
  mode: 0                         # SPI mode (0-3)

polling:
  interval_ms: 100                # Polling interval in milliseconds

buttons:
  - button: 0        # Button to monitor
    # Fire events on: SPIButtonState::OnChange (0x20) | SPIButtonState::OnHold (0x40), lamp toggle feature SPIButtonState::Toggle (0x08)
    config: 0x68
    description: "Button 1"   # Event description
    command: "echo pressed"   # Shell command to execute
```

### Configuration Details

- **button**: Number indicating position on parallel to serial pin of shift register
- **config**: Hex value of enabled features on button
- **command**: Any shell command that will be executed when the trigger matches
- **interval_ms**: How frequently to poll the SPI device

## Installation

### Automated Installation

```bash
chmod +x install.sh
sudo ./install.sh
```

This will:
1. Build the release binary
2. Install to `/usr/local/bin/spi-button-controller`
3. Install configuration to `/etc/spi-button-controller/config.yaml`
4. Install systemd service to `/etc/systemd/system/`
5. Reload systemd daemon

### Manual Installation

```bash
# Build
cargo build --release

# Install binary
sudo install -m 755 target/release/spi-button-controller /usr/local/bin/

# Create config directory
sudo mkdir -p /etc/spi-button-controller

# Install configuration
sudo install -m 644 examples/config.yaml /etc/spi-button-controller/config.yaml

# Install systemd service
sudo install -m 644 systemd/spi-button-controller.service /etc/systemd/system/

# Reload systemd
sudo systemctl daemon-reload
```

## Usage

### Starting the Daemon

```bash
# Start once
sudo systemctl start spi-button-controller

# Enable to start on boot
sudo systemctl enable spi-button-controller

# Check status
sudo systemctl status spi-button-controller
```

### Viewing Logs

```bash
# View recent logs
sudo journalctl -u spi-button-controller -n 50

# Follow logs in real-time
sudo journalctl -u spi-button-controller -f

# View logs since boot
sudo journalctl -u spi-button-controller -b
```

### Reloading Configuration

After editing `/etc/spi-button-controller/config.yaml`:

```bash
sudo systemctl reload spi-button-controller
```

This sends SIGHUP to the daemon, which reloads the configuration without restarting.

### Stopping the Daemon

```bash
sudo systemctl stop spi-button-controller

# Or to disable auto-start:
sudo systemctl disable spi-button-controller
```

## Running Standalone

You can also run the daemon directly without systemd:

```bash
sudo /usr/local/bin/spi-button-controller /etc/spi-button-controller/config.yaml
```

Or with custom configuration:

```bash
sudo /usr/local/bin/spi-button-controller /path/to/custom/config.yaml
```

## Examples

### Basic Button Controller

Monitor buttons on an SPI shift register and execute local commands:

```yaml
spi:
  device: "/dev/spidev0.0"
  speed_hz: 1000000
  mode: 0

polling:
  interval_ms: 100

buttons:
  - button: 0
    config: 0x68  # OnChange | OnHold | Toggle
    description: "Button 1 - Restart Nginx"
    command: "systemctl restart nginx"
  
  - button: 1
    config: 0x68
    description: "Button 2 - Run Script"
    command: "/usr/local/bin/custom-script.sh"
  
  - button: 2
    config: 0x68
    description: "Button 3 - Logger"
    command: "logger -t spi 'Button 3 pressed'"
```

### Button Controller with Klipper Integration

Monitor buttons and send commands to a Klipper API server:

```yaml
spi:
  device: "/dev/spidev0.0"
  speed_hz: 1000000
  mode: 0

polling:
  interval_ms: 100

buttons:
  - button: 0
    config: 0x68
    description: "Home XY Axes"
    command: "klipper:printer.gcode.script|{\"script\":\"G28 X Y\"}"
  
  - button: 1
    config: 0x68
    description: "Heat Bed to 60C"
    command: "klipper:printer.gcode.script|{\"script\":\"M140 S60\"}"
  
  - button: 2
    config: 0x68
    description: "Local Emergency Stop"
    command: "/usr/local/bin/emergency-stop.sh"

klipper:
  socket_path: "/run/klipper_uds"
```

### Mixed System and Klipper Commands

Combine local system commands with Klipper API calls:

```yaml
spi:
  device: "/dev/spidev0.0"
  speed_hz: 1000000
  mode: 0

polling:
  interval_ms: 100

buttons:
  - button: 0
    config: 0x68
    description: "Start Print and Log"
    command: "klipper:printer.print.start|{}"
  
  - button: 1
    config: 0x68
    description: "Stop Print and Notify"
    command: "logger -t spi 'Print stopped'; klipper:printer.print.cancel|{}"
  
  - button: 2
    config: 0x68
    description: "Query Printer Status"
    command: "klipper:printer.objects.query|{\"objects\":{\"printer\":null}}"

klipper:
  socket_path: "/run/klipper_uds"
```

### Button Configuration Details

- **button**: Integer ID of the button on the shift register (0-based)
- **config**: Hex value specifying button behavior flags:
  - `0x20` — OnChange: trigger when button state changes
  - `0x40` — OnHold: trigger when button is held
  - `0x08` — Toggle: toggle mode (lamp control)
  - Combine with bitwise OR, e.g. `0x68` = OnChange | OnHold | Toggle
- **description**: Human-readable label for the button
- **command**: Shell command to execute locally, or `klipper:METHOD|<JSON>` to send to Klipper API

## Architecture

### Main Components

1. **Main Loop** (`src/main.rs`)
   - Initializes logging and configuration
   - Handles signal management (SIGTERM, SIGINT, SIGHUP)
   - Main event loop

2. **Daemon** (`src/daemon.rs`)
   - Manages the polling loop
   - Tracks register state changes
   - Handles debouncing
   - Processes triggers

3. **Command Executor** (`src/command.rs`)
   - Executes shell commands
   - Handles command output and errors
   - Optional timeout support

4. **Configuration** (`src/config.rs`)
   - Data structures for configuration
   - YAML deserialization

## Troubleshooting

### SPI Device Not Found

```
Error: SPI device not found: /dev/spidev0.0
```

## Klipper API Integration

This project includes support for sending commands to a Klipper API server alongside traditional system commands. Key points:

- **Klipper API support**: An optional `klipper` section can be added to the YAML configuration (see `src/config.rs`). Fields:
  - **socket_path**: Path to the Klipper API Unix domain socket, e.g. `/run/klipper_uds`

- **Command types**:
  - **System commands**: Existing behavior — any shell command in the `command` field is executed locally.
  - **Klipper commands**: Commands that start with the prefix `klipper:` are sent to the Klipper API server via Unix domain socket.
    - Syntax: `klipper:METHOD|<JSON_PARAMS>`
    - Example: `klipper:printer.gcode.script|{"script":"G28"}`

- **Request/response flow**:
  1. When a Klipper command is triggered, the daemon generates a `request_id` and immediately sends an `Issued` event (containing `request_id` and trigger metadata) into the internal response queue.
  2. The Klipper request is posted as a JSON-RPC-like object to the configured `klipper.base_url` with the provided method and params.
  3. When the HTTP response arrives, an `EventResponse` is queued with the `request_id`, success status, HTTP status code, and parsed response body.
  4. The main loop maintains a `pending` map of `request_id -> trigger_info` and uses it to correlate responses to the originating button trigger. Once correlated, the mapping is removed and the response is logged.

- **Files involved**:
  - `src/config.rs` — defines `KlipperConfig` and adds optional `klipper` field to `Config`.
  - `src/command.rs` — provides `EventMessage` enum, `EventResponse` struct, and `send_klipper_command` async function (uses `reqwest` for HTTP and `serde_json` for JSON handling).
  - `src/daemon.rs` — accepts an optional response sender, emits `Issued` events before dispatching Klipper requests, and preserves system-command behavior.
  - `src/main.rs` — creates the response queue, tracks pending requests in a map, and correlates incoming responses to original triggers.

- **Build & run**: Build with `cargo build --release` and run with a config path. If using `klipper:` commands, ensure `klipper.base_url` is configured.


Solution: 
- Check if SPI kernel module is loaded: `lsmod | grep spi`
- Check device permissions: `ls -la /dev/spidev*`
- Load SPI module: `sudo modprobe spi_bcm2835` (for Raspberry Pi)

### Permission Denied

```
Error: Failed to open SPI device: /dev/spidev0.0
```

Solution:
- Run with `sudo`
- Or add user to `spi` group: `sudo usermod -a -G spi $USER`

### Configuration Reload Not Working

If `systemctl reload spi-button-controller` doesn't work:

```bash
# Check if service supports reload
sudo systemctl show spi-button-controller -p ReloadSignal

# Manually send SIGHUP
sudo kill -HUP $(systemctl show -p MainPID --value spi-button-controller)
```

### High CPU Usage

If the daemon is using high CPU:
- Increase `polling.interval_ms` in configuration
- Check if commands are hanging (add timeout)
- Check for excessive logging

## Performance Tuning

- **Polling interval**: Increase `polling.interval_ms` for lower CPU usage but higher latency
- **SPI speed**: Increase `speed_hz` for faster communication (depends on device capability)

## Development

### Running Tests

```bash
cargo test
```

### Building Documentation

```bash
cargo doc --open
```

### Development Build

```bash
cargo build
./target/debug/spi-button-controller examples/config.yaml
```

## License

GPL V2.0

## Support

For issues or questions, please create an issue in the repository.

## BeagleBone Black Notes

For BeagleBone Black (BBB), ensure the device tree overlay enables SPI:

```bash
# Check available overlays
ls /boot/dtbs/$(uname -r)/am335x-boneblack-overlay/*.dtb

# Enable SPI overlay (varies by OS)
# Debian/Ubuntu: Edit /boot/uEnv.txt or use universal overlays
# See BBB documentation for your specific OS
```

Common SPI device paths on BBB:
- `/dev/spidev1.0` - SPI1, CS0
- `/dev/spidev1.1` - SPI1, CS1
