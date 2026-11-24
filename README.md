# Agentic Android

<p align="left">
  <a href="https://github.com/jankammerath/AgenticAndroid/releases/latest/download/AgenticAndroid.apk">
    <img src="https://img.shields.io/badge/Download_APK-Android-3b82f6?style=for-the-badge&logo=android&logoColor=white" alt="Download Agentic Android APK" /></a>
  <a href="https://github.com/jankammerath/AgenticAndroid/releases/latest/download/agentic-android.deb">
    <img src="https://img.shields.io/badge/Download_DEB-Debian%2FUbuntu-d70a53?style=for-the-badge&logo=debian&logoColor=white" alt="Download DEB Package" /></a>
  <a href="https://github.com/jankammerath/AgenticAndroid/releases/latest/download/agentic-android.rpm">
    <img src="https://img.shields.io/badge/Download_RPM-Fedora%2FRHEL-ee0000?style=for-the-badge&logo=redhat&logoColor=white" alt="Download RPM Package" /></a>
  <a href="https://github.com/jankammerath/AgenticAndroid/releases">
    <img src="https://img.shields.io/badge/All_Releases-GitHub-1f2937?style=for-the-badge&logo=github&logoColor=white" alt="All Releases" /></a>
</p>

[![GitHub Sponsor](https://img.shields.io/badge/sponsor-30363D?style=flat&logo=GitHub-Sponsors&logoColor=ea4aaa)](https://github.com/sponsors/jankammerath)

Agentic Android is an app built in **Rust** with a minimal **Kotlin** app frontend, serving an embedded webserver with a modern web interface. It uses Android's Accessibility Services to observe and control the phone, exposing human-like interactions — tap, double tap, long press, swipe, text input, app launching, and system navigation — to your configured AI agent.

![Agentic Android shopping groceries](screenshots/agenticandroid-shopping.png)

![Agentic Android planning travel](screenshots/agenticandroid-travel.png)

## Architecture

- **Core Component (`rust-core`)**: High-performance NDK library (`libagentic_android.so`) written in Rust. Runs the Axum HTTP/WebSocket webserver, embedded Web UI, live video/audio streaming hub, session storage, and multi-provider AI agent engine.
- **App UI (`app`)**: Minimal Kotlin/Android frontend whose only role is configuring and starting/stopping the background webserver foreground service, enabling Accessibility Services, and optionally starting high-FPS MediaProjection screen & audio capture.
- **Web UI**: Remote control dashboard combining a multi-session chatbot (with live model discovery for **Claude**, **Gemini**, **Grok**, and **OpenAI**) alongside a live interactive stream of the device screen and audio.

---

## How to Sideload the APK

### Option 1: Install directly on your Android phone

1. **Download the APK** on your Android device using the **[Download APK](https://github.com/jankammerath/AgenticAndroid/releases/latest/download/AgenticAndroid.apk)** button above (or from the [Releases](https://github.com/jankammerath/AgenticAndroid/releases) page).
2. **Allow unknown sources**: Tap the downloaded `AgenticAndroid.apk` file. If Android prompts you, tap **Settings** and enable **Allow from this source** for your browser or file manager, then tap **Install**.
3. **Allow Restricted Settings (Android 13+)**: Because Android restricts sideloaded apps from enabling Accessibility Services by default:
   - Open Android **Settings → Apps → See all apps → Agentic Android**.
   - Tap the **⋮** (three dots) menu in the top-right corner and select **Allow restricted settings**.
4. **Enable Accessibility Control**:
   - Open **Agentic Android** and tap **Enable Accessibility Service** (or go to **Settings → Accessibility → Agentic Android Control**).
   - Toggle **Use Agentic Android Control** on.
5. **Start the Webserver**:
   - In **Agentic Android**, verify the port (default `8080`) and ensure the server is running.
   - *(Optional)* Tap **Start High-FPS Screen & Audio Cast** and approve screen capture for smooth live video and audio streaming. (Without this, the app automatically falls back to Accessibility screenshots on Android 11+.)
6. **Open the Web UI**:
   - Open the `http://<phone-ip>:8080` URL shown in the app from any browser on the same Wi-Fi network.
   - Click **Settings** to enter one or more API keys (**Claude**, **Gemini**, **Grok**, or **OpenAI**).
   - Pick your provider and live-fetched model at the bottom of the prompt box and start chatting!

### Option 2: Sideload via ADB from a computer

```bash
# Install the APK
adb install -r AgenticAndroid.apk

# Forward the webserver port to localhost
adb forward tcp:8080 tcp:8080
```

Then open `http://localhost:8080` in your computer's browser.

---

## Linux Server Installation (Zero-Conf Installer & Upgrader)

### One-Command Install or Upgrade (Debian/Ubuntu & Fedora/RHEL)

Run `install.sh` on any Linux `x86_64` or `arm64` server (e.g., an AWS EC2 `c8i.xlarge` with nested virtualization enabled):

```bash
curl -fsSL https://raw.githubusercontent.com/jankammerath/AgenticAndroid/main/install.sh | sudo bash
```

The installer is **zero-conf** and idempotent (re-running it upgrades to the latest release without overwriting your credentials or config):
1. Installs KVM, Java 17, and all Android emulator runtime libraries automatically.
2. Installs/updates the Android SDK, emulator, and matching system image in `/opt/android-sdk`.
3. Downloads and installs/upgrades the latest `.deb` or `.rpm` package and starts `agentic-android.service` via `systemd`.
4. Prompts for the **port** (default `8080`) and **hostname** — if a hostname is provided, installs **Caddy** and automatically provisions & renews **Let's Encrypt HTTPS** (`https://<hostname>`).
5. On a fresh install, automatically configures a default **username (`admin`) and generated password** (without MFA) and prints them at the end.

For non-interactive installs, you can pass environment variables:

```bash
curl -fsSL https://raw.githubusercontent.com/jankammerath/AgenticAndroid/main/install.sh | \
  sudo PORT=8080 HOSTNAME_DOMAIN=android.example.com LETSENCRYPT_EMAIL=you@example.com bash
```

---

### Manual DEB / RPM Installation & Dependencies

The `.deb` and `.rpm` packages install the `agentic-android` Rust CLI (`/usr/bin/agentic-android`), the bundled universal APK (`/usr/share/agentic-android/AgenticAndroid.apk`), a default config (`/etc/agentic-android/config.toml`), and a `systemd` service (`agentic-android.service`) that runs a headless Android emulator in the background.

### 1. Install KVM & System Dependencies

The Android emulator requires hardware virtualization (**KVM**) on Linux (`x86_64`) plus a few runtime libraries and Java 17 (for `sdkmanager` / `avdmanager`):

**Debian / Ubuntu:**
```bash
sudo apt-get update
sudo apt-get install -y \
  qemu-system-x86 libvirt-daemon-system bridge-utils cpu-checker \
  openjdk-17-jre-headless unzip curl \
  libpulse0 libgl1 libnss3 libxcomposite1 libxcursor1 libxi6 libxtst6

# Verify KVM hardware virtualization is available
kvm-ok
```

**Fedora / RHEL / Rocky Linux:**
```bash
sudo dnf install -y \
  qemu-kvm libvirt java-17-openjdk-headless unzip curl \
  pulseaudio-libs mesa-libGL nss libXcomposite libXcursor libXi libXtst

# Verify /dev/kvm is present
ls -l /dev/kvm
```

### 2. Install Android SDK, Emulator & System Image

Install the Android command-line tools into `/opt/android-sdk` (the default `android_home` in `/etc/agentic-android/config.toml`):

```bash
sudo mkdir -p /opt/android-sdk/cmdline-tools
cd /tmp
curl -fsSL https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip -o cmdline-tools.zip
unzip -q cmdline-tools.zip -d cmdline-tools-tmp
sudo mv cmdline-tools-tmp/cmdline-tools /opt/android-sdk/cmdline-tools/latest
rm -rf cmdline-tools.zip cmdline-tools-tmp

export ANDROID_HOME=/opt/android-sdk
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$PATH"

# Accept licenses and install platform-tools, emulator, and an x86_64 Google APIs system image
yes | sudo "$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" --sdk_root="$ANDROID_HOME" --licenses >/dev/null
sudo "$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" --sdk_root="$ANDROID_HOME" \
  "platform-tools" \
  "emulator" \
  "platforms;android-35" \
  "system-images;android-35;google_apis;x86_64"
```

### 3. Install the Agentic Android Package

**Debian / Ubuntu (`.deb`):**
```bash
curl -fsSL https://github.com/jankammerath/AgenticAndroid/releases/latest/download/agentic-android.deb -o agentic-android.deb
sudo apt-get install -y ./agentic-android.deb
```

**Fedora / RHEL / Rocky Linux (`.rpm`):**
```bash
curl -fsSL https://github.com/jankammerath/AgenticAndroid/releases/latest/download/agentic-android.rpm -o agentic-android.rpm
sudo dnf install -y ./agentic-android.rpm
```

### 4. Configure & Enable the systemd Service

1. *(Optional)* Review `/etc/agentic-android/config.toml` to customize the AVD name, port (default `8080`), or `android_home`. If the configured AVD does not exist yet, `agentic-android` will automatically create `agentic_android_avd` from the installed `x86_64` system image on first boot.
2. Enable and start the service:
   ```bash
   sudo systemctl enable --now agentic-android
   ```
3. Check service status and logs:
   ```bash
   sudo systemctl status agentic-android
   sudo journalctl -u agentic-android -f
   agentic-android daemon status
   ```
4. Open `http://<server-ip>:8080` (or `http://localhost:8080`), add your AI provider keys in **Settings**, and consider enabling **Settings → Security** (username/password + 2FA) if exposed on a network.

---

## Telegram Bot Remote Control

Agentic Android includes a built-in Telegram bot (powered by Rust long-polling — no port forwarding or webhooks required) so you can control your Android device from anywhere over Wi-Fi or cellular.

### Setup

1. **Create a bot**: Open Telegram, message [@BotFather](https://t.me/BotFather), send `/newbot`, and follow the prompts to copy your **Bot Token** (`123456789:AAH...`).
2. **Connect in Agentic Android**: Open the Web UI → **Settings → Telegram**, paste your **Bot Token**, check **Enable Telegram Bot**, and click **Save Telegram Settings**. The status badge will change to `Connected (@your_bot)`.
3. **Authorize your Chat ID**: Open your new bot in Telegram and send `/id`. Copy the numeric Chat ID it replies with, paste it into **Allowed Chat IDs** in **Settings → Telegram** (comma-separated if adding multiple IDs), and click **Save Telegram Settings**.
4. **Control your phone**: Send any natural-language task to your bot — responses and any captured/cropped screenshots are sent right back in Telegram, and the conversation is synced in the Web UI Sessions list.

### Bot Commands

| Command | Description |
| --- | --- |
| `/id` or `/start` | Display your Telegram Chat ID and authorization status |
| `/status` | Show device name, Accessibility status, and default provider/model |
| `/screenshot` | Capture and send the current Android screen as a photo |
| `/new` | Start a fresh agent session for this Telegram chat |
| `/stop` | Stop the currently running agent task |

---

## Development & Running in the Android Emulator

### Prerequisites

- **Rust** (`rustup`) with Android targets (e.g. `aarch64-linux-android` on Apple Silicon or `x86_64-linux-android` on x86_64)
- **Android SDK & NDK** (NDK `27.x`)
- **JDK 17+** (or Android Studio's bundled JBR)

### Rust CLI & Headless Daemon (`cli/`)

Agentic Android includes a Rust CLI (`agentic-android`, wrapped by `./launch.sh`) that builds from source or installs a prebuilt APK, launches an Android emulator (GUI or headless), grants permissions, enables Accessibility Services, forwards the port, and can run as a supervised background daemon on macOS or Linux:

```bash
# Build the Rust CLI (or run ./launch.sh which builds it automatically)
cargo build --release --manifest-path cli/Cargo.toml

# Build from source and launch in the emulator
./launch.sh --avd Pixel_9a --port 8080

# Terminate any running emulator (and stop daemon if active)
./launch.sh stop

# Install from a prebuilt APK instead of building from source
./launch.sh --apk AgenticAndroid.apk --avd Pixel_9a

# Run headless as a background supervisor daemon (ideal for Linux servers)
./launch.sh daemon start --headless --avd Pixel_9a --apk AgenticAndroid.apk --port 8080
./launch.sh daemon status
./launch.sh daemon stop

# Generate a Linux systemd service unit for headless operation
./launch.sh daemon systemd --avd Pixel_9a --apk /opt/agentic/AgenticAndroid.apk --port 8080

# List available AVDs
./launch.sh avds
```

You can also store defaults in `~/.agentic-android/config.toml` (or pass `--config <path>`):

```toml
avd = "Pixel_9a"
apk = "/opt/agentic/AgenticAndroid.apk"
port = 8080
headless = true
android_home = "/opt/android-sdk"
```

Once running, open **http://localhost:8080** in your browser.

---

## Releasing a New Version

Pushing a version tag triggers `.github/workflows/release.yml`, which compiles the Rust core for `arm64-v8a`, `armeabi-v7a`, and `x86_64`, builds a signed universal APK, and publishes a GitHub Release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

You can also trigger a release manually from the **Actions → Build & Release APK** workflow tab.

### Optional Persistent Signing Secrets

To sign release APKs with your own keystore (recommended so users can upgrade in-place without reinstalling), add these repository secrets in **GitHub → Settings → Secrets and variables → Actions**:

- `ANDROID_KEYSTORE_BASE64`: Base64-encoded `.jks` / `.keystore` file (`base64 -i release.keystore`)
- `ANDROID_KEYSTORE_PASSWORD`: Keystore password
- `ANDROID_KEY_ALIAS`: Key alias
- `ANDROID_KEY_PASSWORD`: Key password

If these secrets are not set, the workflow automatically generates an ad-hoc signing key so the published APK is still immediately sideloadable.
