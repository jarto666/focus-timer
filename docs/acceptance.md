# MVP acceptance and perfboard handoff

Final breadboard checkpoint: 2026-08-13. This report maps the four OpenSpec
capabilities to reproducible host tests and recorded target-hardware evidence.
The active MVP is the USB-powered ESP32-C3, SSD1306 OLED, EC11 encoder, and
current-limited 3 V active buzzer. The external WS2812 ring is not connected.

## Automated checkpoint

The final checkpoint ran `./scripts/check-device.sh` from the repository root,
which executes the documented commands from `device/`:

- `cargo fmt --all --check`: pass.
- Core Clippy with warnings denied: pass.
- Firmware host Clippy with warnings denied: pass.
- `focus-core`: 20 behavior tests passed.
- `focus-firmware`: 3 buzzer, 9 input, 5 presentation, and 8 settings tests
  passed.
- All doc-tests passed; total named tests: 45 passed, 0 failed.
- The default ESP32-C3 production firmware build passed after the host suite.

Capability coverage is split as follows:

| Capability | Primary automated evidence | Target evidence |
| --- | --- | --- |
| `preset-selection` | catalog, wraparound, immutable snapshot, and one-press tests in `device/crates/focus-core/tests/behavior.rs` | integrated lifecycle and persistent-selection runs in `docs/hardware.md` |
| `focus-session` | lifecycle, deadline, pause, overflow, completion-idempotence, and no-op tests in `device/crates/focus-core/tests/behavior.rs` | integrated, offline, power-cycle, and user-attested duration runs in `docs/hardware.md` |
| `device-interface` | input, presentation, and buzzer suites in `device/crates/focus-firmware/tests/` | EC11, OLED, buzzer, and integrated lifecycle runs in `docs/hardware.md` |
| `device-settings` | settings adapter suite plus core restore/fallback tests | NVS save, clear, corrupt, reboot, and fallback-start runs in `docs/hardware.md` |

## Breadboard UX checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Encoder access and reliability | Pass | exact slow and fast clockwise/counter-clockwise counts, plus button classification, are recorded in the EC11 bring-up |
| Start within five seconds without documentation | Pass, user accepted | the ready screen exposes one preset and `Press to start`; repeated bench flows selected and started with rotation plus one press; no raw stopwatch trace was retained |
| Paused-state clarity | Pass | the OLED rendered `PAUSED`, a frozen remainder, and `Hold to cancel`; the user exercised pause and resume |
| Long-press discoverability and reliability | Pass | the paused hint exposes the action; an 800 ms hold repeatedly produced one `LongPress`, no extra `Press`, and immediate cancellation |
| OLED readability | Pass | all four upright layouts were readable at normal desk distance; the temporary Dupont/header contact is a mechanical transfer issue, not a layout failure |
| Completion volume | Pass | the user heard the distinct three-pulse completion cadence during the complete offline lifecycle |
| External LED ring | Deferred by design | the oversized reworked ring is disconnected; no temporary mechanical mockup is required for MVP acceptance |

## Scenario audit: preset selection

| Scenario | Result | Evidence |
| --- | --- | --- |
| Prototype catalog is available | Pass | `default_catalog_has_required_presets_in_order` verifies all five IDs, names, order, and durations |
| Invalid catalog entry is detected | Pass | `catalog_rejects_empty_and_bad_default` and `catalog_rejects_invalid_entries` cover blank/duplicate/zero/out-of-range data |
| Rotate to the next preset | Pass | `idle_selection_wraps_and_requests_persistence`; integrated OLED/encoder run |
| Wrap past the first preset | Pass | host wrap test plus forward/reverse physical wraparound run |
| Start the displayed preset | Pass | `one_press_starts_an_immutable_preset_snapshot`; repeated physical one-press starts |
| Meet the common-flow interaction target | Pass, user accepted | breadboard UX checklist above |
| Rotation during a session | Pass | `unsupported_events_are_safe_no_ops_in_every_state`; physical Running/Paused rotation run |

## Scenario audit: focus session

| Scenario | Result | Evidence |
| --- | --- | --- |
| Start from Idle | Pass | one-press core test and physical Start run |
| Pause Running | Pass | `running_session_pauses_and_resumes_without_counting_paused_time`; physical pause run |
| Resume Paused | Pass | same host and physical lifecycle evidence |
| Cancel without confirmation | Pass | `long_press_cancels_running_and_paused_sessions`; physical long-press cancellation |
| Dismiss completion | Pass | `completion_press_returns_to_idle_with_selection_retained`; physical completion dismissal |
| Running time elapses | Pass | `countdown_uses_deadline_and_clamps_at_zero`; visible-second OLED run |
| Time passes while paused | Pass | pause test and roughly ten-second physical paused hold with frozen `00:04` |
| Resume after pause | Pass | pause/resume test proves paused time is excluded |
| Prototype accuracy gate | Pass, user attested | user accepted uninterrupted 15-, 25-, and 90-minute runs within one second; raw timestamps were not retained |
| Countdown reaches zero | Pass | deadline test plus physical `COMPLETE / 00:00` and audible cadence |
| Later timer refreshes | Pass | `completion_effect_is_emitted_exactly_once` |
| No connectivity exists | Pass | complete lifecycle powered only by a USB power bank with no companion or network |
| Reboot during a running session | Pass | Running and Paused power-cycle runs both returned to Idle |

## Scenario audit: device interface

| Scenario | Result | Evidence |
| --- | --- | --- |
| One encoder detent | Pass | `full_detents_emit_exactly_one_event_in_each_direction`; exact physical slow/fast counts |
| Short press | Pass | `short_press_is_emitted_once_after_debounced_release`; physical button run |
| Long press | Pass | held/release long-press tests; physical button run |
| Contact bounce | Pass | quadrature and switch bounce tests |
| Idle screen | Pass | `oled_models_make_all_states_unambiguous`; physical `READY` frames |
| Running screen | Pass | same presentation test; visible physical countdown |
| Paused screen | Pass | same presentation test; physical frozen `PAUSED` frame |
| Completed screen | Pass | same presentation test; physical `COMPLETE / 00:00` frame |
| Session starts | Pass | core Start effect, buzzer cadence test, and physical next-frame/short-pulse evidence |
| Session completes | Pass | exactly-once core effect, three-pulse cadence test, and physical Complete evidence |
| Buzzer is unavailable | Pass by architecture | `runtime.rs` logs and disables only the buzzer output after GPIO failure; application state is committed before feedback I/O |
| Display update fails | Pass | physical `BusWriteError` run continued timing and recovered by rendering the newest snapshot |
| Bench UX gate | Pass | consolidated checklist above |

## Scenario audit: device settings

| Scenario | Result | Evidence |
| --- | --- | --- |
| Reboot after changing selection | Pass | `successful_save_decodes_and_restores_the_selected_preset`; physical Reading restore from NVS |
| First boot | Pass | `first_boot_without_a_record_uses_the_default`; physical cleared-record Pomodoro boot |
| Persisted preset no longer exists | Pass | `removed_preset_record_falls_back_without_blocking_start` |
| Settings record is corrupt | Pass | corrupt-record host test plus physical `Decode(BadMagic)` fallback and successful start |
| Storage write fails | Pass | `write_failure_is_not_retried_and_cannot_revert_the_active_selection` |
| Settings are saved during a session | Pass structurally | `settings_record_cannot_contain_session_state`; version-1 encoded record contains only header and preset ID |

All capability scenarios have passing evidence. There are no unmet OpenSpec
capability scenarios at this checkpoint.

This document freezes the pre-BLE offline MVP checkpoint. The later mobile and
BLE change has its own evidence and open physical follow-ups in
[`docs/ble-sync-acceptance.md`](ble-sync-acceptance.md); those follow-ups do not
retroactively change the offline result above.

## Explicit transfer follow-ups

These are mechanical or evidence-quality follow-ups, not failed capability
scenarios:

1. The temporary OLED/header contact can be disturbed. Replace it with inspected
   soldered joints or sockets and repeat the staged smoke test from
   `docs/wiring.md` after transfer.
2. No inline whole-device current trace was retained. Keep the perfboard load and
   USB-only topology identical; capture actual USB and 3.3 V current before
   adding a battery, LED ring, or any other peripheral.
3. Raw timestamps for the accepted 15/25/90-minute runs were not retained. Repeat
   with serial timestamps if reproducible accuracy evidence is required beyond
   this user-attested MVP gate.
4. The external ring, battery operation, enclosure, and custom PCB remain later
   changes. None may inherit the temporary ring power or signal conclusions
   without a new review.
