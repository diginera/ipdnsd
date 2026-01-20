#!/bin/bash
#
# Install ipdnsd as a system service on Linux/macOS
#

set -e

BINARY_NAME="ipdnsd"
SERVICE_NAME="ipdnsd"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "Please run as root (sudo)"
    exit 1
fi

# Find the binary
if [ -f "./target/release/$BINARY_NAME" ]; then
    BINARY_PATH="$(pwd)/target/release/$BINARY_NAME"
elif [ -f "./$BINARY_NAME" ]; then
    BINARY_PATH="$(pwd)/$BINARY_NAME"
elif command -v $BINARY_NAME &> /dev/null; then
    BINARY_PATH="$(which $BINARY_NAME)"
else
    echo "Error: $BINARY_NAME binary not found"
    echo "Please build with 'cargo build --release' or ensure $BINARY_NAME is in PATH"
    exit 1
fi

echo "Using binary: $BINARY_PATH"

# Detect OS and install appropriate service
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "Installing systemd service..."

    # Create service file
    cat > /etc/systemd/system/${SERVICE_NAME}.service << EOF
[Unit]
Description=IP to DNS Updater
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=$BINARY_PATH daemon
Restart=always
RestartSec=10
User=root

[Install]
WantedBy=multi-user.target
EOF

    # Reload systemd
    systemctl daemon-reload

    # Enable and start service
    systemctl enable $SERVICE_NAME
    systemctl start $SERVICE_NAME

    echo "Service installed and started successfully!"
    echo ""
    echo "Useful commands:"
    echo "  systemctl status $SERVICE_NAME   - Check service status"
    echo "  systemctl stop $SERVICE_NAME     - Stop the service"
    echo "  systemctl restart $SERVICE_NAME  - Restart the service"
    echo "  journalctl -u $SERVICE_NAME -f   - View logs"

elif [[ "$OSTYPE" == "darwin"* ]]; then
    echo "Installing launchd service..."

    PLIST_PATH="/Library/LaunchDaemons/com.diginera.ipdnsd.plist"

    # Create launchd plist
    cat > $PLIST_PATH << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.diginera.ipdnsd</string>
    <key>ProgramArguments</key>
    <array>
        <string>$BINARY_PATH</string>
        <string>daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/var/log/ipdnsd.log</string>
    <key>StandardErrorPath</key>
    <string>/var/log/ipdnsd.error.log</string>
</dict>
</plist>
EOF

    # Load the service
    launchctl load $PLIST_PATH

    echo "Service installed and started successfully!"
    echo ""
    echo "Useful commands:"
    echo "  launchctl list | grep ipdnsd           - Check if running"
    echo "  launchctl unload $PLIST_PATH           - Stop the service"
    echo "  tail -f /var/log/ipdnsd.log            - View logs"

else
    echo "Unsupported operating system: $OSTYPE"
    exit 1
fi

echo ""
echo "Before starting, make sure you have:"
echo "  1. Created a config file (see 'ipdnsd config' for location)"
echo "  2. Stored your API credentials with 'ipdnsd set-key <provider>'"
