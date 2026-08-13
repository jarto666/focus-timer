#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
FOUND_PROTOCOL=0

if [ -f "$REPO_ROOT/device/crates/focus-protocol/Cargo.toml" ]; then
  FOUND_PROTOCOL=1
  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required for Rust protocol checks" >&2
    exit 127
  fi
  cd "$REPO_ROOT/device"
  cargo test -p focus-protocol
fi

if [ -f "$REPO_ROOT/packages/device-protocol/package.json" ]; then
  FOUND_PROTOCOL=1
  if ! command -v pnpm >/dev/null 2>&1; then
    echo "pnpm is required for TypeScript protocol checks" >&2
    exit 127
  fi
  cd "$REPO_ROOT"
  pnpm --filter @focus-timer/device-protocol check
fi

if [ "$FOUND_PROTOCOL" -eq 0 ]; then
  echo "protocol implementations are not initialized yet; no protocol checks to run"
fi
