#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

if [ ! -f "$REPO_ROOT/apps/mobile/package.json" ]; then
  echo "mobile workspace is not initialized yet; no mobile checks to run"
  exit 0
fi

if ! command -v pnpm >/dev/null 2>&1; then
  echo "pnpm is required for mobile checks" >&2
  exit 127
fi

cd "$REPO_ROOT"
pnpm --filter @focus-timer/mobile check
pnpm --filter @focus-timer/device-client check
pnpm --filter @focus-timer/mock-device check
