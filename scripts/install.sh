#!/bin/bash
set -e

PREFIX="${PREFIX:-/opt/omspbase}"
echo "Installing OMSPBase Remote Host to $PREFIX..."

# Create directory structure
mkdir -p "$PREFIX/bin" "$PREFIX/etc" "$PREFIX/web" "$PREFIX/logs"

# Copy binary
cp omspbase-remote-host "$PREFIX/bin/"
chmod +x "$PREFIX/bin/omspbase-remote-host"

# Copy default config (don't overwrite existing)
if [ ! -f "$PREFIX/etc/host.conf" ]; then
    cp host.conf "$PREFIX/etc/host.conf"
    echo "Default config created: $PREFIX/etc/host.conf"
else
    echo "Existing config preserved: $PREFIX/etc/host.conf"
fi

# Register systemd service
if command -v systemctl &> /dev/null; then
    cp omspbase-remote-host.service /etc/systemd/system/
    systemctl daemon-reload
    systemctl enable omspbase-remote-host
    echo "systemd service registered. Edit $PREFIX/etc/host.conf then:"
    echo "  systemctl start omspbase-remote-host"
else
    echo "No systemd found. Start manually:"
    echo "  $PREFIX/bin/omspbase-remote-host --config $PREFIX/etc/host.conf"
fi

echo "Installation complete."
