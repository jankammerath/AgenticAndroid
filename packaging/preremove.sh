#!/bin/sh
set -e

if command -v systemctl >/dev/null 2>&1; then
  systemctl stop agentic-android.service >/dev/null 2>&1 || true
  systemctl disable agentic-android.service >/dev/null 2>&1 || true
fi
