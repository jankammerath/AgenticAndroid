#!/bin/sh
set -e

mkdir -p /var/lib/agentic-android/.android/avd

if command -v systemctl >/dev/null 2>&1; then
  systemctl daemon-reload || true
fi

echo ""
echo "Agentic Android installed!"
echo "  1. Ensure Android SDK, Emulator, and an x86_64 system image are installed in /opt/android-sdk"
echo "  2. Edit /etc/agentic-android/config.toml if needed"
echo "  3. Enable & start the service: sudo systemctl enable --now agentic-android"
echo "  4. Check logs: sudo journalctl -u agentic-android -f"
echo ""
