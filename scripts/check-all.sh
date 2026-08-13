#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

"$SCRIPT_DIR/check-device.sh"
"$SCRIPT_DIR/check-protocol.sh"
"$SCRIPT_DIR/check-mobile.sh"
