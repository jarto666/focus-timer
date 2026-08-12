## 1. Prototype Contract and Workspace

- [x] 1.1 Preserve the photograph confirming the delivered `ESP32-C3-MINI-1` module, connect the board, run `espflash board-info`, and record any identifiable carrier-board vendor/revision, flash size, roles of both USB-C connectors, USB/serial path, onboard-RGB GPIO, and complete pinout before finalizing the GPIO map.
- [x] 1.2 Record the purchased interaction hardware and starter-kit inventory, including the SSD1306 OLED, EC11, standalone and kit active buzzers, passive buzzer, 24-pixel WS2812 ring, optional breadboard power module, wiring/headers, 100 nF/10 uF/100 uF capacitors, resistor values, buttons/LEDs, `74HC595`, and `4N35`; verify actual module pin orders and markings.
- [x] 1.3 Confirm USB as the initial controller power input, identify the board's 5 V/VBUS path and current constraints, inspect the optional power module's model/current/backfeed behavior, and define a single-source breadboard power budget with common ground and no unreviewed parallel supplies.
- [x] 1.4 Define and review the active MVP GPIO/wiring diagram covering 3.3 V OLED and EC11 signals, buzzer load/driver decision, USB power and common ground, boot-strapping constraints, and avoidance of the onboard RGB/USB/serial pins; explicitly show the external WS2812 ring as disconnected/deferred and acquire extra buzzer drive/protection parts only when inspection or testing proves they are required.
- [x] 1.5 Create the Cargo workspace with `crates/focus-core` and `crates/focus-firmware`, keeping the core free of ESP32 dependencies and compatible with `no_std`.
- [x] 1.6 Add repository documentation placeholders for the exact Rust toolchain, target, ESP-IDF version, dependency versions, host test, build, flash, and serial-monitor commands so each is filled by its proving task.

## 2. Host-Tested Core Behavior

- [x] 2.1 Implement stable preset identifiers, preset/catalog validation, the five default prototype presets, cyclic idle selection, and immutable active-preset snapshots in `focus-core`.
- [x] 2.2 Implement the Idle, Running, Paused, and Completed session model plus the explicit Press, LongPress, RotateLeft, RotateRight, and time-advance transition table.
- [x] 2.3 Implement monotonic-deadline remaining-time calculation, pause snapshots, zero clamping, overflow validation, and exactly-once completion effects.
- [x] 2.4 Implement immutable application snapshots and best-effort effect values for rendering, start/completion feedback, persistence, and diagnostic reporting without performing I/O in the core.
- [x] 2.5 Implement settings-record validation and boot selection logic for valid, absent, corrupt, unsupported-version, and removed-preset cases; keep active-session state out of the record.
- [x] 2.6 Add host unit tests covering every state/event transition, catalog edge and wrap cases, time jumps, pause exclusion, cancellation, power-on state, completion idempotence, and settings fallback.
- [x] 2.7 Run and document the host `fmt`, `clippy`, and test commands with all core checks passing before hardware behavior is integrated.

## 3. ESP32-C3 Toolchain and Runtime Skeleton

- [x] 3.1 Configure `focus-firmware` for `std + ESP-IDF` with target `riscv32imc-esp-espidf` on the delivered ESP32-C3-MINI-1-based board and pin the proven Rust toolchain, ESP-IDF release, and crate versions.
- [x] 3.2 Build, flash, and boot a minimal firmware that reports its version and reset reason through serial logging; after its GPIO is identified, use the onboard RGB LED only as an optional bring-up diagnostic.
- [x] 3.3 Implement the monotonic-clock adapter and a single state-owning event-loop skeleton that can process scheduled refresh events without display, storage, or connectivity dependencies.
- [x] 3.4 Verify repeatable clean build, flash, and serial-monitor commands from the repository documentation on the development machine.

## 4. EC11 Rotary Encoder Input

- [x] 4.1 Verify the delivered EC11/module pull-ups and pin order at 3.3 V, then implement the polling-based quadrature decoder and emit exactly one RotateLeft or RotateRight event for each stable detent.
- [x] 4.2 Implement EC11 switch debouncing and mutually exclusive Press/LongPress classification with named, documented starting thresholds.
- [x] 4.3 Add host tests using captured/synthetic GPIO sequences for clockwise and counter-clockwise rotation, contact bounce, short press, long press, and ambiguous transitions.
- [x] 4.4 Run a serial-log bench diagnostic on the wired encoder, record duplicate/missed event counts across slow and fast turns, and tune or replace polling only if the acceptance behavior fails.

## 5. OLED and Audible Feedback

- [x] 5.1 Bring up the purchased 128x64 I2C SSD1306 OLED at the reviewed voltage; record its pin order, bus address, pull-ups, orientation, and readable text result from normal desk distance.
- [x] 5.2 Implement the OLED snapshot-to-view mapping and Idle, Running, Paused, and Completed layouts with preset name, duration or remaining time, and unambiguous state indication.
- [x] 5.3 Render the OLED immediately after state changes and at visible second boundaries, retry transient failures from the newest snapshot, and verify display I/O does not block monotonic accounting.
- [x] 5.4 Preserve the completed 24-pixel WS2812 electrical bring-up and diagnostic evidence for a future hardware version, then leave the reworked oversized ring disconnected from the MVP circuit.
- [x] 5.5 Preserve the completed host-tested ring presentation experiment and diagnostic binaries for possible later reuse without integrating them into the MVP runtime or acceptance path.
- [x] 5.6 Select the standalone 3 V active buzzer unless the kit active buzzer validates better, verify current and any required driver/protection, implement distinct best-effort on/off Start and Complete cadences, keep the passive buzzer out of the MVP path, and confirm a disconnected or failed buzzer does not change session state.

## 6. End-to-End Offline Timer

- [x] 6.1 Connect semantic input, scheduled time events, core snapshots/effects, OLED rendering, buzzer playback, and serial diagnostics through the single state-owning event loop, with no external LED-ring dependency.
- [x] 6.2 Verify on-device selection, wraparound, one-press start, pause, resume, long-press cancel without confirmation, completion, completion dismissal, and ignored rotation during an active session.
- [ ] 6.3 Measure uninterrupted 15-, 25-, and 90-minute sessions against a reference clock and record that active-duration completion error is at most one second.
- [x] 6.4 Verify the complete session lifecycle after boot with radios unprovisioned and no phone, computer companion, network, cloud service, or wall-clock source present.
- [x] 6.5 Power-cycle the device from Running and Paused states and verify it returns safely to Idle without restoring an active session.

## 7. Persistent Selection

- [x] 7.1 Implement the version-1 non-volatile settings adapter containing only the schema version and selected preset identifier.
- [x] 7.2 Coalesce selection saves after a quiet period, avoid writes when the identifier is unchanged, and preserve the in-memory selection when a write fails.
- [x] 7.3 Add adapter/core boundary tests for successful restore, first boot, corrupt bytes, unsupported version, removed preset, and simulated write failure.
- [ ] 7.4 Validate on hardware that a saved selection survives reboot, erased/corrupt settings fall back to the documented default, and every fallback still permits a session to start.

## 8. Acceptance and Handoff

- [ ] 8.1 Finalize the hardware and developer documentation with exact board/module markings, starter-kit inventory disposition, USB/VBUS and optional power-module decision, GPIO table, OLED/encoder/buzzer wiring, explicit external-ring deferral, measured active-MVP power budget, any conditionally added buzzer interface/protection parts, thresholds, pinned versions, and tested build/test/flash/monitor commands.
- [ ] 8.2 Run the full host test suite and capture a concise passing summary tied to the four capability specs.
- [ ] 8.3 Run the breadboard UX checklist for encoder access and reliability, start within five seconds without documentation, paused-state clarity, long-press discoverability, OLED readability, and completion volume; record the oversized external ring as deferred rather than requiring a temporary mechanical mockup.
- [ ] 8.4 Audit the prototype against every scenario in `preset-selection`, `focus-session`, `device-interface`, and `device-settings`; record evidence or an explicit follow-up for any unmet scenario before archiving the change.
