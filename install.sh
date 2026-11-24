#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Agentic Android - Zero-Conf Linux Installer & Upgrader
# Supports: Debian / Ubuntu (.deb) and Fedora / RHEL / Rocky / Alma (.rpm)
# ==============================================================================

REPO="jankammerath/AgenticAndroid"
ANDROID_HOME="/opt/android-sdk"
CONFIG_FILE="/etc/agentic-android/config.toml"
STATE_DIR="/var/lib/agentic-android"

if [ "$(uname -s)" != "Linux" ]; then
  echo "ERROR: install.sh is intended for Linux servers. On macOS, use ./launch.sh instead." >&2
  exit 1
fi

if [ "$(id -u)" -ne 0 ]; then
  echo "ERROR: Please run install.sh as root (e.g. sudo ./install.sh or curl ... | sudo bash)." >&2
  exit 1
fi

# Detect package manager
PKG_MGR=""
if command -v apt-get >/dev/null 2>&1; then
  PKG_MGR="apt"
elif command -v dnf >/dev/null 2>&1; then
  PKG_MGR="dnf"
else
  echo "ERROR: Unsupported Linux distribution (neither apt-get nor dnf found)." >&2
  exit 1
fi

# Detect CPU architecture for Android system image
HOST_ARCH="$(uname -m)"
if [ "$HOST_ARCH" = "aarch64" ] || [ "$HOST_ARCH" = "arm64" ]; then
  ANDROID_ABI="arm64-v8a"
else
  ANDROID_ABI="x86_64"
fi

# Helper for interactive prompts (works even when piped via `curl | sudo bash`)
prompt_input() {
  local prompt_text="$1"
  local default_val="$2"
  local var_name="$3"
  local input=""

  if [ -r /dev/tty ]; then
    printf "%s [%s]: " "$prompt_text" "$default_val" >/dev/tty
    IFS= read -r input </dev/tty || true
  fi

  if [ -z "$input" ]; then
    printf -v "$var_name" '%s' "$default_val"
  else
    printf -v "$var_name" '%s' "$input"
  fi
}

echo "============================================================"
echo " Agentic Android - Linux Installer & Upgrader"
echo "============================================================"

# Read existing port from config.toml if upgrading
EXISTING_PORT="8080"
if [ -f "$CONFIG_FILE" ]; then
  DETECTED_PORT="$(grep -E '^\s*port\s*=' "$CONFIG_FILE" | head -n 1 | sed -E 's/.*=\s*([0-9]+).*/\1/' || true)"
  if [[ "$DETECTED_PORT" =~ ^[0-9]+$ ]]; then
    EXISTING_PORT="$DETECTED_PORT"
  fi
fi

# Read existing hostname from Caddyfile if upgrading
EXISTING_HOSTNAME=""
if [ -f /etc/caddy/Caddyfile ]; then
  EXISTING_HOSTNAME="$(awk '/^[A-Za-z0-9.-]+\.[A-Za-z]{2,}\s*\{/{print $1; exit}' /etc/caddy/Caddyfile 2>/dev/null || true)"
fi

if [ -z "${PORT:-}" ]; then
  prompt_input "Webserver port" "$EXISTING_PORT" PORT
fi
if ! [[ "$PORT" =~ ^[0-9]+$ ]] || [ "$PORT" -lt 1024 ] || [ "$PORT" -gt 65535 ]; then
  echo "ERROR: Invalid port '$PORT'. Must be between 1024 and 65535." >&2
  exit 1
fi

if [ -z "${HOSTNAME_DOMAIN+x}" ]; then
  prompt_input "Hostname for Let's Encrypt HTTPS (leave blank for HTTP only)" "$EXISTING_HOSTNAME" HOSTNAME_DOMAIN
fi

LETSENCRYPT_EMAIL="${LETSENCRYPT_EMAIL:-}"
if [ -n "$HOSTNAME_DOMAIN" ] && [ -z "$LETSENCRYPT_EMAIL" ]; then
  prompt_input "Email for Let's Encrypt renewal notices (optional)" "" LETSENCRYPT_EMAIL
fi

echo ""
echo "==> [1/6] Installing KVM, Java, and Android emulator dependencies ($PKG_MGR)..."
if [ "$PKG_MGR" = "apt" ]; then
  export DEBIAN_FRONTEND=noninteractive
  apt-get update -y
  if [ "$ANDROID_ABI" = "arm64-v8a" ]; then
    QEMU_PKG="qemu-system-arm"
  else
    QEMU_PKG="qemu-system-x86"
  fi
  apt-get install -y \
    ca-certificates curl unzip openssl \
    "$QEMU_PKG" libvirt-daemon-system bridge-utils \
    openjdk-17-jre-headless \
    libpulse0 libgl1 libnss3 libxcomposite1 libxcursor1 libxi6 libxtst6
else
  dnf install -y \
    ca-certificates curl unzip openssl \
    qemu-kvm libvirt \
    java-17-openjdk-headless \
    pulseaudio-libs mesa-libGL nss libXcomposite libXcursor libXi libXtst
fi

if [ ! -e /dev/kvm ]; then
  echo "WARNING: /dev/kvm was not found. Hardware virtualization (KVM) is strongly recommended" >&2
  echo "         for Android emulator performance (e.g. AWS EC2 c8i/m8i with nested virtualization)." >&2
fi

echo "==> [2/6] Installing/updating Android SDK, Emulator & system image ($ANDROID_ABI)..."
mkdir -p "$ANDROID_HOME/cmdline-tools"
SDKMANAGER="$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager"
if [ ! -x "$SDKMANAGER" ]; then
  TMP_DIR="$(mktemp -d)"
  curl -fsSL "https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip" -o "$TMP_DIR/cmdline-tools.zip"
  unzip -q "$TMP_DIR/cmdline-tools.zip" -d "$TMP_DIR/unpacked"
  rm -rf "$ANDROID_HOME/cmdline-tools/latest"
  mv "$TMP_DIR/unpacked/cmdline-tools" "$ANDROID_HOME/cmdline-tools/latest"
  rm -rf "$TMP_DIR"
fi

yes | "$SDKMANAGER" --sdk_root="$ANDROID_HOME" --licenses >/dev/null || true
"$SDKMANAGER" --sdk_root="$ANDROID_HOME" \
  "platform-tools" \
  "emulator" \
  "platforms;android-35" \
  "system-images;android-35;google_apis;${ANDROID_ABI}"

echo "==> [3/6] Downloading & installing/upgrading latest Agentic Android package..."
TMP_PKG_DIR="$(mktemp -d)"
if [ "$PKG_MGR" = "apt" ]; then
  PKG_FILE="$TMP_PKG_DIR/agentic-android.deb"
  curl -fsSL "https://github.com/${REPO}/releases/latest/download/agentic-android.deb" -o "$PKG_FILE"
  apt-get install -y "$PKG_FILE"
else
  PKG_FILE="$TMP_PKG_DIR/agentic-android.rpm"
  curl -fsSL "https://github.com/${REPO}/releases/latest/download/agentic-android.rpm" -o "$PKG_FILE"
  dnf install -y "$PKG_FILE" || dnf upgrade -y "$PKG_FILE"
fi
rm -rf "$TMP_PKG_DIR"

echo "==> [4/6] Updating $CONFIG_FILE and starting systemd service..."
mkdir -p /etc/agentic-android "$STATE_DIR/.android/avd"
if [ -f "$CONFIG_FILE" ]; then
  if grep -qE '^\s*port\s*=' "$CONFIG_FILE"; then
    sed -i -E "s/^\s*port\s*=.*/port = ${PORT}/" "$CONFIG_FILE"
  else
    printf "\nport = %s\n" "$PORT" >> "$CONFIG_FILE"
  fi
else
  cat > "$CONFIG_FILE" <<EOF
avd = "agentic_android_avd"
apk = "/usr/share/agentic-android/AgenticAndroid.apk"
port = ${PORT}
headless = true
android_home = "${ANDROID_HOME}"
state_dir = "${STATE_DIR}"
EOF
fi

systemctl daemon-reload
systemctl enable agentic-android.service
systemctl restart agentic-android.service

echo "    Waiting for headless emulator & Agentic Android webserver on port ${PORT}..."
echo "    (First boot creates the AVD and may take 1-2 minutes)"
SERVER_READY=0
for _ in $(seq 1 90); do
  if curl -fsS "http://127.0.0.1:${PORT}/api/health" >/dev/null 2>&1; then
    SERVER_READY=1
    break
  fi
  sleep 2
done

echo "==> [5/6] Configuring default login credentials (no MFA)..."
CREATED_DEFAULT_LOGIN=0
DEFAULT_USER="${DEFAULT_USER:-admin}"
DEFAULT_PASSWORD="${DEFAULT_PASSWORD:-}"

if [ "$SERVER_READY" -eq 1 ]; then
  AUTH_STATUS="$(curl -fsS "http://127.0.0.1:${PORT}/api/auth/status" 2>/dev/null || true)"
  if echo "$AUTH_STATUS" | grep -q '"auth_enabled":false'; then
    if [ -z "$DEFAULT_PASSWORD" ]; then
      DEFAULT_PASSWORD="$(openssl rand -base64 18 | tr -dc 'A-Za-z0-9' | head -c 16)"
    fi
    curl -fsS -X POST "http://127.0.0.1:${PORT}/api/auth/credentials" \
      -H "Content-Type: application/json" \
      -d "{\"username\":\"${DEFAULT_USER}\",\"password\":\"${DEFAULT_PASSWORD}\"}" >/dev/null
    CREATED_DEFAULT_LOGIN=1
    echo "    Default login configured for user '${DEFAULT_USER}' (MFA disabled)."
  else
    echo "    Existing login credentials detected; keeping current auth configuration."
  fi
else
  echo "WARNING: Webserver did not become ready within 180s; skipping default login seeding." >&2
  echo "         Check logs with: journalctl -u agentic-android -f" >&2
fi

echo "==> [6/6] Configuring HTTPS with Let's Encrypt..."
if [ -n "$HOSTNAME_DOMAIN" ]; then
  if ! command -v caddy >/dev/null 2>&1; then
    if [ "$PKG_MGR" = "apt" ]; then
      apt-get install -y debian-keyring debian-archive-keyring apt-transport-https gpg
      curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor --yes -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
      curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' > /etc/apt/sources.list.d/caddy-stable.list
      apt-get update -y
      apt-get install -y caddy
    else
      dnf install -y 'dnf-command(copr)' || true
      dnf copr enable -y @caddy/caddy || true
      dnf install -y caddy
    fi
  fi

  mkdir -p /etc/caddy
  {
    if [ -n "$LETSENCRYPT_EMAIL" ]; then
      echo "{"
      echo "    email ${LETSENCRYPT_EMAIL}"
      echo "}"
      echo ""
    fi
    echo "${HOSTNAME_DOMAIN} {"
    echo "    reverse_proxy 127.0.0.1:${PORT}"
    echo "}"
  } > /etc/caddy/Caddyfile

  systemctl enable caddy.service
  systemctl restart caddy.service
  echo "    Caddy configured for https://${HOSTNAME_DOMAIN} -> 127.0.0.1:${PORT} (automatic Let's Encrypt)."
else
  echo "    No hostname provided; skipping Let's Encrypt HTTPS setup."
fi

echo ""
echo "============================================================"
echo " Agentic Android installation / upgrade complete!"
echo "============================================================"
if [ -n "$HOSTNAME_DOMAIN" ]; then
  echo " Web UI (HTTPS): https://${HOSTNAME_DOMAIN}"
  echo " Ensure DNS A/AAAA record for ${HOSTNAME_DOMAIN} points here"
  echo " and ports 80 & 443 are open for Let's Encrypt."
else
  echo " Web UI (HTTP):  http://localhost:${PORT}"
fi
if [ "$CREATED_DEFAULT_LOGIN" -eq 1 ]; then
  echo ""
  echo " Default Login (no MFA):"
  echo "   Username: ${DEFAULT_USER}"
  echo "   Password: ${DEFAULT_PASSWORD}"
  echo " (You can change your password or enable 2FA in Settings -> Security)"
fi
echo ""
echo " Useful commands:"
echo "   sudo systemctl status agentic-android"
echo "   sudo journalctl -u agentic-android -f"
echo "   agentic-android daemon status"
echo "============================================================"
