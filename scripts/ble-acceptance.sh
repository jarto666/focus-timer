#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
FIRMWARE_DIR="$REPO_ROOT/device/crates/focus-firmware"
FIRMWARE_ELF="$REPO_ROOT/device/target/riscv32imc-esp-espidf/debug/focus-firmware"

mode=production
port=auto
flash=false
monitor=false
evidence_root=

usage() {
  cat <<'EOF'
usage: ./scripts/ble-acceptance.sh [options]

Build one evidence-labelled firmware image. Flashing and serial monitoring are
opt-in and require exactly one explicit or auto-detected native USB port.

options:
  --mode MODE            production (default), acceptance, ble-echo,
                         ble-faults, radio-failure, journal-fill, journal-corrupt,
                         or journal-clear
  --port PORT            /dev/cu.usbmodem... or "auto" (default)
  --flash                inspect the board and flash the selected image
  --monitor              monitor after flashing and save the serial transcript
  --evidence-root PATH   parent directory for the timestamped evidence run
  -h, --help             show this help

Examples:
  ./scripts/ble-acceptance.sh --mode ble-echo
  ./scripts/ble-acceptance.sh --mode ble-echo --flash --monitor
  ./scripts/ble-acceptance.sh --mode acceptance --port /dev/cu.usbmodem101 \
    --flash --monitor

The script never checks OpenSpec tasks automatically. A human must review the
saved log and record the observed physical behavior before accepting a task.
EOF
}

fail() {
  echo "BLE acceptance: $*" >&2
  exit 64
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --mode)
      [ "$#" -ge 2 ] || fail "--mode requires a value"
      mode=$2
      shift 2
      ;;
    --port)
      [ "$#" -ge 2 ] || fail "--port requires a value"
      port=$2
      shift 2
      ;;
    --flash)
      flash=true
      shift
      ;;
    --monitor)
      monitor=true
      flash=true
      shift
      ;;
    --evidence-root)
      [ "$#" -ge 2 ] || fail "--evidence-root requires a value"
      evidence_root=$2
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

case "$mode" in
  production)
    cargo_arguments="--offline"
    ;;
  acceptance)
    cargo_arguments="--offline --features acceptance-diagnostic"
    ;;
  ble-faults)
    cargo_arguments="--offline --features ble-fault-diagnostic"
    ;;
  radio-failure)
    cargo_arguments="--offline --features radio-failure-diagnostic"
    ;;
  ble-echo)
    cargo_arguments="--offline --no-default-features --features ble-echo-diagnostic"
    ;;
  journal-fill)
    cargo_arguments="--offline --no-default-features --features journal-fill-diagnostic"
    ;;
  journal-corrupt)
    cargo_arguments="--offline --no-default-features --features journal-corrupt-diagnostic"
    ;;
  journal-clear)
    cargo_arguments="--offline --no-default-features --features journal-clear-diagnostic"
    ;;
  *)
    fail "unsupported mode '$mode'"
    ;;
esac

command -v cargo >/dev/null 2>&1 || {
  echo "cargo is required; source the Rust and ESP environments from docs/development.md" >&2
  exit 127
}

if [ "$flash" = true ]; then
  command -v espflash >/dev/null 2>&1 || {
    echo "espflash is required; source the ESP environment from docs/development.md" >&2
    exit 127
  }

  if [ "$port" = auto ]; then
    set -- /dev/cu.usbmodem*
    [ "$1" != '/dev/cu.usbmodem*' ] || fail "no /dev/cu.usbmodem controller is connected"
    [ "$#" -eq 1 ] || fail "multiple controllers found; select one with --port"
    port=$1
  fi

  case "$port" in
    /dev/cu.usbmodem*) ;;
    *) fail "refusing non-native-USB port '$port'; expected /dev/cu.usbmodem..." ;;
  esac
  [ -e "$port" ] || fail "serial port does not exist: $port"
fi

echo "BLE acceptance: building mode=$mode"
cd "$FIRMWARE_DIR"
# Intentional word splitting: this variable contains only the fixed arguments
# selected by the closed mode list above, never user-provided shell text.
# shellcheck disable=SC2086
cargo build $cargo_arguments
[ -f "$FIRMWARE_ELF" ] || fail "expected firmware artifact is missing: $FIRMWARE_ELF"

if [ -z "$evidence_root" ]; then
  if [ "$flash" = true ]; then
    evidence_root="$REPO_ROOT/docs/hardware-evidence/ble-sync"
  else
    evidence_root="${TMPDIR:-/tmp}/focus-timer-ble-acceptance"
  fi
fi

timestamp=$(date -u '+%Y%m%dT%H%M%SZ')
run_dir="$evidence_root/$timestamp-$mode"
commit=$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || echo unknown)
git_status=$(git -C "$REPO_ROOT" status --porcelain 2>/dev/null || true)
if [ -n "$git_status" ]; then
  worktree=dirty
else
  worktree=clean
fi
[ ! -e "$run_dir" ] || fail "evidence run already exists: $run_dir"
mkdir -p "$run_dir"
artifact="$run_dir/focus-firmware-$mode"
cp "$FIRMWARE_ELF" "$artifact"

checksum=$(shasum -a 256 "$artifact" | awk '{print $1}')
bytes=$(wc -c < "$artifact" | tr -d ' ')

{
  echo "schema=focus-timer-ble-acceptance-v1"
  echo "started_at_utc=$timestamp"
  echo "mode=$mode"
  echo "git_commit=$commit"
  echo "git_worktree=$worktree"
  echo "artifact=focus-firmware-$mode"
  echo "artifact_bytes=$bytes"
  echo "artifact_sha256=$checksum"
  if [ "$flash" = true ]; then
    echo "port=$port"
  else
    echo "port=not-requested"
  fi
} > "$run_dir/run.env"
printf '%s\n' "$git_status" > "$run_dir/git-status.txt"
cat > "$run_dir/observations.md" <<EOF
# BLE acceptance observation

- Run: \`$timestamp-$mode\`
- Firmware mode: \`$mode\`
- Commit: \`$commit\`
- Port: \`$(if [ "$flash" = true ]; then printf '%s' "$port"; else printf '%s' 'not flashed'; fi)\`

## Visible behavior

- [ ] OLED remained responsive.
- [ ] Encoder rotation and button gestures remained correct.
- [ ] Start, pause, resume, cancel, complete, and dismiss behaved correctly.
- [ ] Buzzer feedback remained correct.
- [ ] iPhone discovery/connection/synchronization observation recorded below.
- [ ] Disconnect/reconnect and advertising restart observation recorded below.
- [ ] Journal/result observation recorded below.
- [ ] Production firmware was restored after diagnostics.

## Measurements

| Measurement | Value | How observed |
| --- | --- | --- |
| Idle free heap | not measured | |
| Connected free heap | not measured | |
| Transfer minimum free heap | not measured | |
| Negotiated ATT MTU/value bytes | not measured | |
| Logical response bytes/time | not measured | |
| Watchdog or reset count | not measured | |
| Whole-device USB current | not measured | |

## Notes

Not reviewed yet. This generated template is not acceptance evidence by itself.
EOF

echo "BLE acceptance: artifact and manifest saved in $run_dir"

if [ "$flash" = false ]; then
  echo "BLE acceptance: build-only complete; no controller was inspected or flashed"
  exit 0
fi

echo "BLE acceptance: inspecting $port"
if espflash board-info --port "$port" --before usb-reset --non-interactive \
  > "$run_dir/board-info.log" 2>&1; then
  cat "$run_dir/board-info.log"
else
  status=$?
  cat "$run_dir/board-info.log" >&2
  exit "$status"
fi

echo "BLE acceptance: flashing mode=$mode to $port"
if espflash flash --port "$port" --before usb-reset "$artifact" \
  > "$run_dir/flash.log" 2>&1; then
  cat "$run_dir/flash.log"
else
  status=$?
  cat "$run_dir/flash.log" >&2
  exit "$status"
fi

if [ "$monitor" = false ]; then
  echo "BLE acceptance: flash complete; serial monitor was not requested"
  exit 0
fi

echo "BLE acceptance: monitoring $port; press Ctrl-C after the physical scenario"
echo "BLE acceptance: serial transcript is $run_dir/monitor.log"
espflash monitor --port "$port" 2>&1 | tee "$run_dir/monitor.log"
