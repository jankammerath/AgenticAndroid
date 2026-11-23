#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PORT="${PORT:-8080}"
AVD_NAME="${AVD_NAME:-}"
BUILD_ONLY="${BUILD_ONLY:-0}"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --avd|-a)
      AVD_NAME="${2:?Missing value for $1}"
      shift 2
      ;;
    --port|-p)
      PORT="${2:?Missing value for $1}"
      shift 2
      ;;
    --build-only)
      BUILD_ONLY=1
      shift
      ;;
    --help|-h)
      echo "Usage: ./launch.sh [--avd <AVD_NAME>] [--port <PORT>] [--build-only]"
      echo "Env vars: AVD_NAME, PORT (default 8080), ABI, BUILD_ONLY=1"
      exit 0
      ;;
    *)
      echo "Unknown option: $1 (use --help for usage)" >&2
      exit 1
      ;;
  esac
done

# Prefer rustup toolchain in ~/.cargo/bin
if [ -d "$HOME/.cargo/bin" ]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

# Detect ANDROID_HOME
if [ -z "${ANDROID_HOME:-}" ]; then
  if [ -d "$HOME/Library/Android/sdk" ]; then
    export ANDROID_HOME="$HOME/Library/Android/sdk"
  elif [ -d "$HOME/Android/Sdk" ]; then
    export ANDROID_HOME="$HOME/Android/Sdk"
  else
    echo "ERROR: ANDROID_HOME not found. Please set ANDROID_HOME." >&2
    exit 1
  fi
fi
export ANDROID_SDK_ROOT="$ANDROID_HOME"

# Detect JAVA_HOME (prefer Android Studio JBR if system java is missing)
if [ -z "${JAVA_HOME:-}" ] || [ ! -x "${JAVA_HOME}/bin/java" ]; then
  if [ -x "/Applications/Android Studio.app/Contents/jbr/Contents/Home/bin/java" ]; then
    export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
  elif command -v /usr/libexec/java_home >/dev/null 2>&1 && /usr/libexec/java_home -v 17 >/dev/null 2>&1; then
    export JAVA_HOME="$(/usr/libexec/java_home -v 17)"
  fi
fi

# Detect NDK
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  NDK_DIR="$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | sort -V | tail -n 1 || true)"
  if [ -z "$NDK_DIR" ]; then
    echo "ERROR: Android NDK not found in $ANDROID_HOME/ndk. Install via sdkmanager." >&2
    exit 1
  fi
  export ANDROID_NDK_HOME="$NDK_DIR"
fi

ADB="$ANDROID_HOME/platform-tools/adb"
EMULATOR="$ANDROID_HOME/emulator/emulator"
AVDMANAGER="$ANDROID_HOME/cmdline-tools/latest/bin/avdmanager"

# Determine ABI matching host arch unless overridden
HOST_ARCH="$(uname -m)"
if [ "$HOST_ARCH" = "arm64" ] || [ "$HOST_ARCH" = "aarch64" ]; then
  ABI="${ABI:-arm64-v8a}"
  RUST_TARGET="aarch64-linux-android"
  NDK_CLANG_PREFIX="aarch64-linux-android29"
  CARGO_LINKER_VAR="CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"
  CC_VAR="CC_aarch64_linux_android"
  AR_VAR="AR_aarch64_linux_android"
else
  ABI="${ABI:-x86_64}"
  RUST_TARGET="x86_64-linux-android"
  NDK_CLANG_PREFIX="x86_64-linux-android29"
  CARGO_LINKER_VAR="CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER"
  CC_VAR="CC_x86_64_linux_android"
  AR_VAR="AR_x86_64_linux_android"
fi

# Detect NDK prebuilt host tag
if [ "$(uname -s)" = "Darwin" ]; then
  NDK_HOST_TAG="darwin-x86_64"
else
  NDK_HOST_TAG="linux-x86_64"
fi
TOOLCHAIN_BIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_HOST_TAG/bin"

echo "==> [1/5] Building Rust NDK core ($RUST_TARGET -> $ABI)..."
if command -v rustup >/dev/null 2>&1; then
  if ! rustup target list --installed | grep -q "^${RUST_TARGET}$"; then
    rustup target add "$RUST_TARGET"
  fi
fi

export "$CARGO_LINKER_VAR=$TOOLCHAIN_BIN/${NDK_CLANG_PREFIX}-clang"
export "$CC_VAR=$TOOLCHAIN_BIN/${NDK_CLANG_PREFIX}-clang"
export "$AR_VAR=$TOOLCHAIN_BIN/llvm-ar"

(
  cd "$ROOT_DIR/rust-core"
  cargo build --release --target "$RUST_TARGET"
)

JNILIBS_DIR="$ROOT_DIR/app/src/main/jniLibs/$ABI"
mkdir -p "$JNILIBS_DIR"
cp "$ROOT_DIR/rust-core/target/$RUST_TARGET/release/libagentic_android.so" "$JNILIBS_DIR/libagentic_android.so"
echo "    Copied libagentic_android.so to $JNILIBS_DIR"

echo "==> [2/5] Building Android APK..."
if [ ! -f "$ROOT_DIR/local.properties" ]; then
  echo "sdk.dir=$ANDROID_HOME" > "$ROOT_DIR/local.properties"
fi

(
  cd "$ROOT_DIR"
  if [ -x "./gradlew" ]; then
    ./gradlew assembleDebug
  else
    GRADLE_BIN="$(find "$HOME/.gradle/wrapper/dists" -name gradle -type f 2>/dev/null | head -n 1 || true)"
    if [ -n "$GRADLE_BIN" ]; then
      "$GRADLE_BIN" assembleDebug
    else
      gradle assembleDebug
    fi
  fi
)

APK_PATH="$ROOT_DIR/app/build/outputs/apk/debug/app-debug.apk"
if [ "$BUILD_ONLY" = "1" ]; then
  echo "Build complete: $APK_PATH"
  exit 0
fi

echo "==> [3/5] Checking Android simulator / emulator..."
RUNNING_SERIAL="$("$ADB" devices | awk 'NR>1 && $2=="device" {print $1; exit}')"

if [ -z "$RUNNING_SERIAL" ]; then
  if [ -z "$AVD_NAME" ]; then
    ALL_AVDS="$("$EMULATOR" -list-avds || true)"
    AVD_NAME="$(echo "$ALL_AVDS" | grep -iE 'pixel|phone' | head -n 1 || true)"
    if [ -z "$AVD_NAME" ]; then
      AVD_NAME="$(echo "$ALL_AVDS" | head -n 1 || true)"
    fi
  fi

  if [ -z "$AVD_NAME" ]; then
    AVD_NAME="agentic_android_avd"
    echo "    No AVD found. Creating '$AVD_NAME'..."
    SYS_IMG="$(find "$ANDROID_HOME/system-images" -mindepth 3 -maxdepth 3 -type d -name "$ABI" 2>/dev/null | sort -V | tail -n 1 || true)"
    if [ -z "$SYS_IMG" ]; then
      echo "ERROR: No system image found for ABI '$ABI' in $ANDROID_HOME/system-images" >&2
      exit 1
    fi
    REL_IMG="${SYS_IMG#"$ANDROID_HOME/"}"
    PKG_ID="${REL_IMG//\//;}"
    echo "    Using system image: $PKG_ID"
    echo "no" | "$AVDMANAGER" create avd --force --name "$AVD_NAME" --package "$PKG_ID" --device "pixel_6"
  fi

  echo "    Starting emulator '$AVD_NAME'..."
  nohup "$EMULATOR" -avd "$AVD_NAME" -no-snapshot-load -no-boot-anim >/tmp/agentic_android_emulator.log 2>&1 &

  echo "    Waiting for emulator to boot..."
  "$ADB" wait-for-device
  for _ in $(seq 1 90); do
    BOOT="$("$ADB" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')"
    if [ "$BOOT" = "1" ]; then
      break
    fi
    sleep 2
  done
  sleep 2
fi

echo "==> [4/5] Installing APK & configuring Accessibility Service..."
"$ADB" install -r "$APK_PATH"

# Grant runtime permissions cleanly
"$ADB" shell pm grant com.agenticandroid android.permission.RECORD_AUDIO 2>/dev/null || true
"$ADB" shell pm grant com.agenticandroid android.permission.POST_NOTIFICATIONS 2>/dev/null || true

# Enable Accessibility Service automatically for simulator convenience
SERVICE_ID="com.agenticandroid/com.agenticandroid.AgentAccessibilityService"
EXISTING_SERVICES="$("$ADB" shell settings get secure enabled_accessibility_services 2>/dev/null | tr -d '\r' || true)"
if [ -z "$EXISTING_SERVICES" ] || [ "$EXISTING_SERVICES" = "null" ]; then
  NEW_SERVICES="$SERVICE_ID"
elif echo "$EXISTING_SERVICES" | grep -q "$SERVICE_ID"; then
  NEW_SERVICES="$EXISTING_SERVICES"
else
  NEW_SERVICES="${EXISTING_SERVICES}:${SERVICE_ID}"
fi
"$ADB" shell settings put secure enabled_accessibility_services "$NEW_SERVICES"
"$ADB" shell settings put secure accessibility_enabled 1

echo "==> [5/5] Forwarding port $PORT and launching Agentic Android..."
"$ADB" forward "tcp:$PORT" "tcp:$PORT"
"$ADB" shell am start -n com.agenticandroid/.MainActivity --ez auto_start true --ei port "$PORT"

echo "    Waiting for Rust webserver on http://localhost:$PORT ..."
for _ in $(seq 1 20); do
  if curl -fsS "http://localhost:$PORT/api/health" >/dev/null 2>&1; then
    echo ""
    echo "============================================================"
    echo " Agentic Android is running in the simulator!"
    echo " Web UI: http://localhost:$PORT"
    echo "============================================================"
    exit 0
  fi
  sleep 1
done

echo "WARNING: App launched, but http://localhost:$PORT/api/health did not respond yet. Check logcat with: $ADB logcat -s AgenticAndroidRust"
