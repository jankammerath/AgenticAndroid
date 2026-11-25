#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLI_BIN="$ROOT_DIR/cli/target/release/agentic-android"

if [ -d "$HOME/.cargo/bin" ]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

if [ ! -x "$CLI_BIN" ] || [ "$ROOT_DIR/cli/src/main.rs" -nt "$CLI_BIN" ] || [ "$ROOT_DIR/cli/Cargo.toml" -nt "$CLI_BIN" ]; then
  cargo build --release --manifest-path "$ROOT_DIR/cli/Cargo.toml"
fi

exec "$CLI_BIN" "$@"
