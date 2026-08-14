# Development

## Proven environment

The initial development machine has these tools installed. Firmware versions
below are pinned by the repository and proven by a successful build.

| Component | Proven version |
| --- | --- |
| Host Rust and Cargo | 1.97.1 |
| ESP Rust toolchain | `esp`, rustc 1.95.0-nightly (2026-04-15) |
| Firmware target | `riscv32imc-esp-espidf` |
| Xtensa GCC | not required by ESP32-C3; installed for possible other ESP32 targets |
| `espup` | 0.17.1 |
| `espflash` | 4.5.0 |
| `ldproxy` | 0.3.5 |
| Ninja | 1.13.2 |
| ESP-IDF | v5.5.3 |
| FreeRTOS tick rate | 1000 Hz (1 ms) |
| `esp-idf-svc` | 0.52.1 |
| `embuild` | 0.33.2 |
| `log` | 0.4.29 |
| `ssd1306` | 0.10.0 |
| `embedded-graphics` | 0.8.2 |

For a new shell:

```sh
source /Users/ellion/.cargo/env
source /Users/ellion/export-esp.sh
```

## Host checks

Run the complete device host suite from the repository root:

```sh
./scripts/check-device.sh
```

The script runs these commands from `device/`. The final pre-move checkpoint
was repeated on 2026-08-14 with host Rust/Cargo 1.97.1:

```sh
cd device
cargo fmt --all --check
cargo clippy -p focus-core --all-targets -- -D warnings
cargo test -p focus-core
cargo clippy -p focus-firmware --no-default-features \
  --target aarch64-apple-darwin --all-targets -- -D warnings
cargo test -p focus-firmware --no-default-features \
  --target aarch64-apple-darwin
```

Checkpoint result: formatting clean, both Clippy invocations clean with warnings
denied, 20 core behavior tests, 3 buzzer-cadence tests, 9 firmware-adapter input
tests, 5 presentation tests, 8 settings adapter/core-boundary tests, and all
doc-tests passing. That is 45 named tests with zero failures. The explicit host
target and disabled ESP-IDF feature let the same EC11 decoder be tested on macOS
while its default build remains the ESP32-C3 firmware. Capability-level mapping
is recorded in `docs/acceptance.md`.

### Pre-monorepo device checkpoint

The device workspace was checkpointed immediately before its move under
`device/` on 2026-08-14. The existing host suite above passed unchanged: 20
`focus-core` behavior tests and 25 `focus-firmware` adapter tests, with both
Clippy runs warning-free and formatting clean. The default ESP32-C3 firmware
also built successfully with ESP-IDF v5.5.3, host Rust/Cargo 1.97.1, ESP Rust
1.95.0-nightly (2026-04-15), and espflash 4.5.0.

The pre-move debug ELF was 16,688,968 bytes with SHA-256
`ca8bd30966dc565cb2850645185e699e516b21ff03dda929fbd0db6b5176af49`.
This is a reproducibility checkpoint, not a flash-size measurement. The
independent `build-focus-device-mvp` change deliberately remains at 39/40:
its whole-device USB-current measurement in task 8.1 is still open.

### Path-migration checkpoint

After moving the Rust workspace to `device/` on 2026-08-14,
`./scripts/check-device.sh` passed the same 45 named tests, formatting, both
warning-denied Clippy runs, and all doc-tests. A clean default ESP32-C3 build
then passed from `device/crates/focus-firmware`, followed by successful builds
of `ring-diagnostic`, `ring-full-diagnostic`, `ring-capped-diagnostic`,
`encoder-diagnostic`, `oled-diagnostic`, `buzzer-diagnostic`,
`acceptance-diagnostic`, `settings-clear-diagnostic`, and
`settings-corrupt-diagnostic`. Journal diagnostics were added after this
path-only checkpoint and are covered separately below. The default firmware
was rebuilt last.

The post-move debug ELF is 16,689,056 bytes with SHA-256
`90fbe07ac7be200ed62a76c8bab2818781a309265f716b8691fd2d46c7d2aff6`.
Its debug-path-dependent hash and size are recorded as the new canonical build
checkpoint; they are not a flash-size or USB-current measurement.

## Firmware commands

Run firmware commands from the firmware crate so its target configuration and
linked `esp` toolchain are selected automatically:

```sh
# Build from the repository root
./scripts/build-firmware.sh

# Build (from device/crates/focus-firmware)
cd device/crates/focus-firmware
cargo build

# Inspect a connected board
espflash board-info --port /dev/cu.usbmodem101 --before usb-reset --non-interactive

# Flash and monitor
espflash flash --port /dev/cu.usbmodem101 --before usb-reset --monitor \
  ../../target/riscv32imc-esp-espidf/debug/focus-firmware
```

The build, flash, and monitor commands were validated twice on 2026-08-08. A
clean build with a new external `CARGO_TARGET_DIR` also passed in 57.79 seconds,
without deleting or relying on the repository's existing artifacts. The boot
evidence is recorded in `docs/hardware.md`.

## Diagnostic firmware layout

Alternative hardware-test entrypoints live under
`device/crates/focus-firmware/src/diagnostics/`. Each diagnostic feature still
builds the complete `focus-firmware` executable, but compile-time `cfg` selection makes
`main` enter exactly one diagnostic run loop instead of the production event
loop. Concrete modes select the internal `diagnostic-firmware` aggregate
transitively, so `main.rs` only needs one production-versus-diagnostic switch.
Do not select that aggregate directly: `diagnostics/mod.rs` owns concrete feature
routing and rejects both a missing concrete mode and incompatible diagnostics
selected together.

The root-level `runtime.rs`, `settings.rs`, and `nvs_settings.rs` are production
modules. `acceptance-diagnostic` is intentionally different from the hardware
diagnostics: it uses the production runtime and adapters with only the preset
durations shortened to eight seconds. All production and diagnostic feature
builds are checked after changing the routing or module layout.

## Archived WS2812 diagnostics (deferred from MVP)

These feature builds are retained as evidence and as a possible starting point
for a later, smaller LED ring. The current prototype must leave the external
ring, GPIO10, and the controller `5V out` disconnected. Do not flash these
diagnostics as part of the current MVP workflow.

The `ring-diagnostic` feature replaces the normal runtime with a deliberately
limited GPIO10/RMT test. It first transmits an all-off frame, waits two seconds,
then alternates one blue pixel at 32/255 with all pixels off. It never illuminates
more than one pixel and must not be used as evidence that the full ring or power
path has passed task 5.4.

Build the diagnostic while the ring's 5 V wire remains disconnected:

```sh
cd device/crates/focus-firmware
cargo build --features ring-diagnostic
espflash flash --port /dev/cu.usbmodem101 --before usb-reset \
  ../../target/riscv32imc-esp-espidf/debug/focus-firmware
```

After visually reviewing `GND`, `DI -> 330 ohm -> GPIO10`, capacitor polarity,
and the absence of a 5 V-to-GND short, disconnect USB, connect ring `5V` to
controller `5V out`, and reconnect USB. Monitor the diagnostic separately:

```sh
espflash monitor --port /dev/cu.usbmodem101
```

Immediately disconnect USB if more than one pixel illuminates, colors are
unstable, the controller resets, a wire or connector warms, or the expected
two-second dim-blue/off cycle is absent.

Only after the one-pixel cycle is visually confirmed, build and flash the
second stage:

```sh
cd device/crates/focus-firmware
cargo build --features ring-full-diagnostic
espflash flash --port /dev/cu.usbmodem101 --before usb-reset \
  ../../target/riscv32imc-esp-espidf/debug/focus-firmware
```

`ring-full-diagnostic` first transmits a startup-off frame, then holds all 24
pixels continuously at clearly visible white `RGB(32,32,32)`. The frame is
resent every two seconds without an off interval. Its traditional worst-case
estimate is about 181 mA for the ring, so it stays inside the reviewed 32/255
per-channel prototype cap; absolute full-white `RGB(255,255,255)` is deliberately
not supported. Disconnect USB immediately if colors differ, pixels flicker, the
controller resets, or any wire or connector warms.

After both low-current stages pass and the local 100 uF electrolytic plus
100 nF (`104`) ceramic capacitors are connected in parallel across ring 5 V and
GND, test the actual application channel cap:

```sh
cd device/crates/focus-firmware
cargo build --features ring-capped-diagnostic
espflash flash --port /dev/cu.usbmodem101 --before usb-reset \
  ../../target/riscv32imc-esp-espidf/debug/focus-firmware
```

This stage starts off and cycles all 24 pixels through the maximum-load frames
used by the application: Running `RGB(0,10,32)`, Paused `RGB(32,8,0)`, and
Completed `RGB(0,32,4)`. Each frame and the all-off interval lasts two seconds.
No channel exceeds the firmware's 32/255 cap. The same disconnect conditions
apply; full-brightness white remains outside the reviewed prototype profile.

## EC11 serial diagnostic

The `encoder-diagnostic` feature replaces the normal runtime with a 1 ms polling
loop on the bench-revised map: `S1 -> GPIO0`, `S2 -> GPIO4`, and
active-low `KEY -> GPIO5`. The module pin labelled `5V` is deliberately powered
from controller `3V3`, not from 5 V, so its fitted pull-ups remain in the ESP32
logic domain. The GPIO inputs also enable the ESP32-C3's internal pull-ups. Bench
testing found that the module's fitted `103` network did not hold both phase
inputs high reliably with floating GPIOs, while the internal pull-ups safely
restored defined 3.3 V logic levels.

The repository pins `CONFIG_FREERTOS_HZ=1000` in `sdkconfig.defaults`. The ESP-IDF
default of 100 Hz rounds a one-millisecond FreeRTOS delay up to a ten-millisecond
tick, which the physical EC11 test showed can miss a short intermediate phase.
At 1000 Hz the polling task yields on every sample without starving the idle
task or triggering its watchdog.

Disconnect USB and ring 5 V before changing wiring. Connect EC11 `5V` to `3V3`,
`KEY` to GPIO5, `S2` to GPIO4, `S1` to GPIO0, and `GND` to common GND. Then build,
flash, and monitor:

```sh
cd device/crates/focus-firmware
cargo build --features encoder-diagnostic
espflash flash --port /dev/cu.usbmodem101 --before usb-reset \
  ../../target/riscv32imc-esp-espidf/debug/focus-firmware
espflash monitor --port /dev/cu.usbmodem101
```

The monitor reports `RotateLeft`, `RotateRight`, `Press`, and `LongPress` plus
cumulative counts. The validated phase orientation maps clockwise to
`RotateRight` and counter-clockwise to `RotateLeft`. Count slow and fast detents
against the log before tasks 4.1 and 4.4 are accepted.

## SSD1306 OLED diagnostic

The `oled-diagnostic` feature replaces the normal runtime with a focused
128x64 display test. It uses the reviewed map `SCK/SCL -> GPIO6` and
`SDA -> GPIO7` at 100 kHz, probes only the two SSD1306 addresses `0x3C` and
`0x3D` with a harmless display-off command, and logs the address that ACKs.
The first physical run with the controller's internal I2C pull-ups disabled
timed out at both candidate addresses. The diagnostic therefore enables the
ESP32-C3's 3.3 V-domain pull-ups. With those pull-ups enabled, the delivered
module ACKs at `0x3C` and cycles all four frames on short breadboard wires. The
module's own pull-up network remains unproven and is not inferred from the
marketplace listing.

Keep USB disconnected while changing wires. The delivered module's front pin
order is `GND`, `VDD`, `SCK`, `SDA`. Connect `GND -> GND`, `VDD -> 3V3`,
`SCK -> GPIO6`, and `SDA -> GPIO7`. Do not connect the OLED to 5 V, and do not
add a series resistor or capacitor to either I2C signal. The EC11 can remain on
GPIO0/GPIO4/GPIO5; the ring 5 V wire can remain disconnected for this focused
test.

Build before powering the newly wired display, then flash and monitor through
the controller's native `USB` connector:

```sh
cd device/crates/focus-firmware
cargo build --features oled-diagnostic
espflash flash --port /dev/cu.usbmodem101 --before usb-reset \
  ../../target/riscv32imc-esp-espidf/debug/focus-firmware
espflash monitor --port /dev/cu.usbmodem101
```

The display cycles `READY`, `FOCUS`, `PAUSED`, and `COMPLETE` frames every three
seconds using the production snapshot-to-view mapping. Acceptance requires one
logged ACK address, upright output, all four layouts visible without clipping,
and readable text from normal desk distance. If the output is upside down,
change only the driver rotation after recording the observed mounting
orientation.

## Integrated OLED, encoder, and buzzer runtime

The default firmware runtime owns one `focus_core::App`, polls the reviewed EC11
map every millisecond, advances the core from a monotonic clock, and renders the
newest immutable snapshot to the OLED. Accepted input requests an immediate
render; a visible-second change requests the next countdown frame. A failed
OLED transfer leaves the render pending and retries the newest snapshot after
250 ms without rolling back or pausing application state. Start and Complete
effects trigger the timestamp-driven GPIO1 buzzer cadence after the core has
committed its transition; buzzer GPIO errors disable only audible feedback. The
runtime does not initialize GPIO10, drive `5V out`, or depend on an external LED
ring.

Build and flash the default runtime without a diagnostic feature:

```sh
cd device/crates/focus-firmware
cargo build
espflash flash --port /dev/cu.usbmodem101 --before usb-reset \
  ../../target/riscv32imc-esp-espidf/debug/focus-firmware
espflash monitor --port /dev/cu.usbmodem101
```

### Short-duration lifecycle acceptance build

The `acceptance-diagnostic` feature runs the integrated production event loop,
OLED, EC11, and buzzer adapters, but substitutes 8-second copies of the five
production presets. IDs, names, order, default selection, transitions, display
mapping, and feedback are unchanged. This build exists only to exercise the
complete on-device lifecycle without waiting for a 15-minute completion; it is
not evidence for the duration-accuracy measurements in task 6.3.

```sh
cd device/crates/focus-firmware
cargo build --features acceptance-diagnostic
espflash flash --port /dev/cu.usbmodem101 --before usb-reset \
  ../../target/riscv32imc-esp-espidf/debug/focus-firmware
espflash monitor --port /dev/cu.usbmodem101
```

The 2026-08-13 run verified forward and reverse catalog wraparound, one-press
start, ignored rotation in Running and Paused, a frozen paused countdown,
resume from the preserved remainder, exactly one completion transition,
audible three-pulse completion feedback, dismissal to Idle, and direct
long-press cancellation to Idle. The serial evidence is summarized in
`docs/hardware.md`. Rebuild without this feature after the lifecycle check to
restore production durations.

### Persistent preset selection

The production runtime opens the default ESP-IDF NVS partition and the
`focus_timer` namespace. One blob under the `settings` key contains the `FT`
magic, schema version 1, preset-ID length, and stable preset-ID bytes. There are
no duration, deadline, pause, completion, or other session-state fields.

Encoder selection changes take effect in core memory immediately. The firmware
waits for a 1,000 ms quiet period before committing the newest identifier, so a
fast traversal of several presets performs one write. Returning to the already
persisted identifier cancels the pending write. A read/decode failure boots the
core with `SettingsLoad::Corrupt`; a write/commit failure is logged after the
core state has already changed and is not retried until a later selection
change.

The host settings suite covers successful restore, missing first-boot data,
corrupt bytes, unsupported schema version, a removed preset identifier,
coalescing, unchanged-selection write avoidance, and a simulated commit
failure that cannot revert selection or prevent session start. Hardware NVS
restore, clear, corrupt fallback, and fallback-session-start paths passed on
2026-08-13; see `docs/hardware.md`.

## Session-journal NVS diagnostics

The journal diagnostics are complete alternative firmware images. They open
only the `focus_sync` namespace and never open or erase the production
`focus_timer` settings namespace. Select exactly one mode at a time, flash it,
observe its single completion log, and rebuild/flash the default firmware
before using the timer again.

```sh
cd device/crates/focus-firmware

# Remove identity, metadata, and slots only from focus_sync.
cargo build --features journal-clear-diagnostic

# Corrupt both metadata copies and slot s00 only in focus_sync.
cargo build --features journal-corrupt-diagnostic

# Append 65 synthetic outcomes, forcing the 64-slot journal to evict once.
cargo build --features journal-fill-diagnostic

# Always restore the production image after the selected experiment.
cargo build
```

Flash a selected build using the standard `espflash flash` command above. The
clear diagnostic logs `JOURNAL CLEAR DIAGNOSTIC`; the next production boot must
provision a fresh identity and epoch while leaving the selected timer preset
unchanged. The corrupt diagnostic logs `JOURNAL CORRUPT DIAGNOSTIC`; the next
production boot must report a degraded recovery (or a clean rotated epoch when
no unambiguous valid slot remains) and still enter the normal input/OLED loop.
The fill diagnostic logs bounds, high-water sequence, and
`stale_cursor_gap=true`; it intentionally logs no session payload. These are
destructive journal tests, so do not run them when retained session history is
valuable.

## Active-buzzer diagnostic

The `buzzer-diagnostic` feature replaces the normal runtime with a repeating
GPIO1 test for the standalone 3 V active buzzer. It emits one 80 ms Start pulse,
waits two seconds, emits a three-pulse Complete cadence, and repeats. The
cadence model is timestamp-driven and contains no sleeps or GPIO access; only
this focused diagnostic loop applies its level changes.

Disconnect USB before wiring. Connect `GPIO1 -> 330 ohm -> buzzer +` and
`buzzer - -> GND`. The resistor limits the theoretical worst-case GPIO current
to approximately `3.3 V / 330 ohm = 10 mA`. Do not substitute the passive
buzzer and do not reduce the resistor if the result is quiet.

```sh
cd device/crates/focus-firmware
cargo build --features buzzer-diagnostic
espflash flash --port /dev/cu.usbmodem101 --before usb-reset \
  ../../target/riscv32imc-esp-espidf/debug/focus-firmware
espflash monitor --port /dev/cu.usbmodem101
```

Both cadences were physically confirmed at usable volume on 2026-08-12. The
current-limited direct path is sufficient for the prototype, so no NPN driver is
required. If a different buzzer is substituted later, repeat this review rather
than reducing the resistor by trial and error.
