#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required; source the Rust environment documented in docs/development.md" >&2
  exit 127
fi

cd "$REPO_ROOT/device"
cargo fmt --all --check
cargo clippy -p focus-core --all-targets -- -D warnings
cargo test -p focus-core
cargo clippy -p focus-firmware --no-default-features \
  --target aarch64-apple-darwin --all-targets -- -D warnings
cargo test -p focus-firmware --no-default-features \
  --target aarch64-apple-darwin
