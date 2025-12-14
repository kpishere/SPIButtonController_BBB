#!/bin/bash

# Installation script for SPI Button Controller daemon

set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
CONFIG_DIR="${CONFIG_DIR:-/etc/spi-button-controller}"
SYSTEMD_DIR="${SYSTEMD_DIR:-/etc/systemd/system}"

echo "Building SPI Button Controller..."
cargo build --release

BINARY_PATH="./target/release/spi-button-controller"

if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    exit 1
fi

echo "Installing binary to $INSTALL_DIR..."
sudo install -m 755 "$BINARY_PATH" "$INSTALL_DIR/spi-button-controller"

echo "Creating configuration directory..."
sudo mkdir -p "$CONFIG_DIR"

echo "Installing configuration file..."
if [ ! -f "$CONFIG_DIR/config.yaml" ]; then
    sudo install -m 644 examples/config.yaml "$CONFIG_DIR/config.yaml"
    echo "  ✓ Configuration installed to $CONFIG_DIR/config.yaml"
else
    echo "  ✓ Configuration already exists, keeping existing file"
fi

echo "Installing systemd service..."
sudo install -m 644 systemd/spi-button-controller.service "$SYSTEMD_DIR/"

echo "Reloading systemd daemon..."
sudo systemctl daemon-reload

echo ""
echo "Installation complete!"
echo ""
echo "To start the service:"
echo "  sudo systemctl start spi-button-controller"
echo ""
echo "To enable auto-start on boot:"
echo "  sudo systemctl enable spi-button-controller"
echo ""
echo "To check service status:"
echo "  sudo systemctl status spi-button-controller"
echo ""
echo "To view logs:"
echo "  sudo journalctl -u spi-button-controller -f"
echo ""
echo "Configuration file location: $CONFIG_DIR/config.yaml"
echo "Edit the configuration and run 'sudo systemctl reload spi-button-controller' to apply changes"
