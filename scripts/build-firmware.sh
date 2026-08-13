#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required; source the Rust and ESP environments documented in docs/development.md" >&2
  exit 127
fi

cd "$REPO_ROOT/device/crates/focus-firmware"
cargo build "$@"
