#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

if [ ! -f "$REPO_ROOT/device/crates/focus-protocol/Cargo.toml" ] || \
   [ ! -f "$REPO_ROOT/packages/device-protocol/package.json" ] || \
   [ ! -f "$REPO_ROOT/protocol/fixtures/logical-v1.txt" ]; then
  echo "Rust, TypeScript, and shared protocol fixtures are all required" >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required for Rust protocol checks" >&2
  exit 127
fi

if ! command -v pnpm >/dev/null 2>&1; then
  echo "pnpm is required for TypeScript protocol checks" >&2
  exit 127
fi

cd "$REPO_ROOT/device"
cargo fmt --all --check
cargo clippy -p focus-protocol --all-targets -- -D warnings
cargo test --locked -p focus-protocol

cd "$REPO_ROOT"
pnpm --filter @focus-timer/device-protocol check
