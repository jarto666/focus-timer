# BLE firmware development

The ESP32-C3 firmware uses the ESP-IDF NimBLE host through the exact
`esp32-nimble = 0.12.0` dependency. This is the smallest maintained Rust
surface tested against the repository's pinned ESP-IDF 5.5 toolchain that
provides server connect/disconnect, write, subscribe, MTU, notification, and
advertising-restart callbacks. Bluedroid is disabled in `sdkconfig.defaults`;
the controller is configured for one peripheral connection.

The private service registry remains canonical in `protocol/schema.md`:

- service `1cf47046-2e37-4642-a30e-df24879f994f`;
- write-with-response command `65ecdf0d-cde0-4543-a62b-c166c3341319`;
- notify response `2c4e304b-2581-481a-8646-89122d760711`;
- local advertising name `FocusTimer`.

Advertising contains only the short name and service UUID. Stable identity,
journal metadata, session data, and hardware address are never advertised.

## Architecture boundary

`ble_radio.rs` is the only production module that imports `esp32-nimble`.
NimBLE callbacks can update only a mutex-protected, fixed-capacity transport
mailbox: connection generation, subscription/MTU state, a 2 KiB reassembler,
and one copied complete request. They cannot access `focus_core::App`, GPIO,
OLED, buzzer, settings, clock anchor, or NVS.

The one-millisecond firmware loop remains the only owner of timer state. It:

1. samples physical input and advances the timer;
2. flushes best-effort settings and journal work;
3. takes at most one complete BLE request from the mailbox;
4. applies the connection-scoped Hello gate and handles only status, page, or
   volatile clock-anchor requests;
5. queues one bounded response and emits at most one notification fragment per
   loop iteration.

The preferred ATT MTU is 185, giving a maximum characteristic value of 182
bytes. Framing also works at the mandatory 20-byte characteristic value. A
partial ingress transfer expires after three seconds. Disconnect increments a
connection generation, clears ingress/outbound state, resets the protocol
handshake, and lets the server restart advertising. Notification failure drops
only that response; read-only clients may retry.

If the durable journal cannot provide a stable device identity, production BLE
is not started. If preferred-MTU or advertising setup fails, the runtime logs
the adapter failure and continues as an offline timer. The selected high-level
crate initializes the underlying NimBLE host through an infallible API; target
watchdog/reset and heap behavior still require the physical acceptance run.

## Echo diagnostic

The echo image proves the GATT lifecycle before involving CBOR or journal data:

```sh
source "$HOME/.cargo/env"
source "$HOME/export-esp.sh"
cd device/crates/focus-firmware
cargo build --offline --no-default-features --features ble-echo-diagnostic
espflash flash --port /dev/cu.usbmodem101 --before usb-reset --monitor \
  ../../target/riscv32imc-esp-espidf/debug/focus-firmware
```

On iPhone, scan for the Focus Timer service, connect to `FocusTimer`, subscribe
to the response characteristic, and write one short value to the command
characteristic. The response notification must contain the same bytes. Then
disconnect and confirm the device appears in a new foreground scan. Logs print
only handles and byte counts, never payload contents.

This source image and the production firmware compile cleanly. Physical iPhone
discovery/write/notify/re-advertise evidence remains open until the board is
connected and the diagnostic is run.

## Reproducible physical acceptance runner

Use the root runner for the remaining physical OpenSpec checks. It accepts a
closed list of production and diagnostic modes, builds offline, copies the
exact ELF into a timestamped evidence directory, and records the commit,
worktree state, checksum, byte size, board information, flash transcript, and
optional serial transcript. Merely building an image never looks for or flashes
a controller:

```sh
source "$HOME/.cargo/env"
source "$HOME/export-esp.sh"

# Safe without hardware: prepare the exact echo image and manifest only.
./scripts/ble-acceptance.sh --mode ble-echo

# With one controller connected to its native USB connector:
./scripts/ble-acceptance.sh --mode ble-echo --flash --monitor

# Short integrated timer lifecycle with production BLE/journal adapters:
./scripts/ble-acceptance.sh --mode acceptance --flash --monitor
```

Supported modes are `production`, `acceptance`, `ble-echo`, `journal-fill`,
`journal-corrupt`, and `journal-clear`. Auto-selection refuses to continue when
zero or multiple `/dev/cu.usbmodem...` devices exist; pass `--port` to resolve
the latter deliberately. `--monitor` implies `--flash`. Evidence defaults to
`docs/hardware-evidence/ble-sync/<UTC timestamp>-<mode>` for physical flashes;
build-only preparation defaults to the system temporary directory so it does
not dirty the repository. Either can be redirected with `--evidence-root`.

The runner deliberately never edits `tasks.md` and never labels a run as
passed. Review the transcript together with the visible OLED, encoder, buzzer,
iPhone, and (for current) meter observations, then record those observations in
the acceptance ledger. Diagnostic images remain installed after their run, so
always finish the sequence by flashing `--mode production`.

`acceptance-diagnostic` alone enables the narrow `focus-esp-resources` adapter.
That isolated crate contains the single audited ESP-IDF FFI block used to read
heap and current-task stack high-water counters; the main workspace retains its
`unsafe_code = "forbid"` policy. Default production features neither compile
nor link the adapter. `check-device.sh` still verifies its formatting, while
the target acceptance build proves the actual ESP symbol linkage.

## Verification commands

```sh
# Host protocol/session/projection tests
./scripts/check-device.sh

# Default integrated target build
./scripts/build-firmware.sh

# Focused target lint, from device/crates/focus-firmware
cargo clippy --offline --bin focus-firmware -- -D warnings
```

Host framing tests round-trip a maximum 2 KiB response at both 20-byte and
182-byte GATT value capacities, verify contiguous offsets/checksum, expire an
abandoned transfer without another incoming fragment, and accept a clean retry
after malformed input.
