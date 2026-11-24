#!/bin/sh
set -e

# On DEB upgrade ($1 = upgrade) or RPM upgrade ($1 >= 1), do not disable the service
case "$1" in
  upgrade|1|2)
    exit 0
    ;;
esac

if command -v systemctl >/dev/null 2>&1; then
  systemctl stop agentic-android.service >/dev/null 2>&1 || true
  systemctl disable agentic-android.service >/dev/null 2>&1 || true
fi
