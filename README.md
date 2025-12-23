# SPI Button Controller Daemon

A Linux daemon service written in Rust that monitors an SPI device for register value changes and executes arbitrary shell commands based on those values.

## Features

- **Async/await based polling** - Non-blocking SPI device monitoring using Tokio
- **Configuration file driven** - YAML configuration for registers and command mappings
- **Event debouncing** - Prevents rapid re-triggering of the same event
- **Graceful shutdown** - Handles SIGTERM and SIGINT signals properly
- **Configuration reload** - Send SIGHUP to reload configuration without restarting
- **Systemd integration** - Runs as a native Linux daemon with journald logging
- **Value masking** - Support for partial register value matching with bit masks
- **Multiple registers** - Monitor and respond to changes in multiple registers
- **Shell command execution** - Execute arbitrary shell commands on register value changes

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

### Button Matrix Controller

Monitor a button matrix on an SPI device and run commands:

```yaml
registers:
  - register_address: 0x00
    name: "Button Matrix"
    value_triggers:
      - value: 0x01
        description: "Button 1 pressed"
        command: "systemctl restart nginx"
      
      - value: 0x02
        description: "Button 2 pressed"
        command: "shutdown -h now"
      
      - value: 0x04
        description: "Button 3 pressed"
        command: "/usr/local/bin/custom-script.sh"
```

### LED Control with Masking

Monitor LED status register with bit masking:

```yaml
registers:
  - register_address: 0x10
    name: "LED Status"
    value_triggers:
      - value: 0x01
        mask: 0x01
        description: "LED 1 on"
        command: "logger -t spi 'LED 1 is now on'"
      
      - value: 0x00
        mask: 0x01
        description: "LED 1 off"
        command: "logger -t spi 'LED 1 is now off'"
```

### Temperature Sensor Status

Monitor device status with multiple triggers:

```yaml
registers:
  - register_address: 0x20
    name: "Device Status"
    value_triggers:
      - value: 0x00
        description: "Normal operation"
        command: "systemctl start cooling-system"
      
      - value: 0x01
        description: "Overheating"
        command: "logger -t spi 'WARNING: Device overheating'; systemctl restart cooling-system"
      
      - value: 0x02
        description: "Critical temperature"
        command: "logger -t spi 'CRITICAL: Device critical temperature'; shutdown -h 5"
```

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

3. **SPI Device** (`src/spi.rs`)
   - Low-level SPI communication
   - Register read/write operations

4. **Command Executor** (`src/command.rs`)
   - Executes shell commands
   - Handles command output and errors
   - Optional timeout support

5. **Configuration** (`src/config.rs`)
   - Data structures for configuration
   - YAML deserialization

## Troubleshooting

### SPI Device Not Found

```
Error: SPI device not found: /dev/spidev0.0
```

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
- **Debounce interval**: Adjust `debounce_ms` based on expected event frequency
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

[Your License Here]

## Contributing

[Contributing Guidelines]

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
- `/dev/spidev2.0` - SPI2, CS0
