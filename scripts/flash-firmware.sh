#!/bin/sh
set -eu

if [ "$#" -lt 1 ]; then
  echo "usage: $0 /dev/cu.usbmodem... [cargo build arguments]" >&2
  exit 64
fi

FOCUS_TIMER_SERIAL_PORT=$1
shift

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

if ! command -v cargo >/dev/null 2>&1 || ! command -v espflash >/dev/null 2>&1; then
  echo "cargo and espflash are required; source the environments documented in docs/development.md" >&2
  exit 127
fi

cd "$REPO_ROOT/device/crates/focus-firmware"
cargo build "$@"
espflash flash --port "$FOCUS_TIMER_SERIAL_PORT" --before usb-reset --monitor \
  ../../target/riscv32imc-esp-espidf/debug/focus-firmware
