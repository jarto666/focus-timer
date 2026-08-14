# BLE acceptance observation

- Run: `20260814T183110Z-radio-failure`
- Firmware mode: `radio-failure`
- Commit: `0c7ebc1146c1e9e8a4dcc3a7259c2b07e6363afd`
- Port: `/dev/cu.usbmodem101`

## Visible behavior

- [x] OLED remained responsive through READY, FOCUS, PAUSED, COMPLETE, and dismiss.
- [x] Slow and fast encoder rotation in both directions, short press, and long press remained correct.
- [x] Start, pause, resume, running cancellation, complete, and dismiss behaved correctly.
- [x] Start and completion buzzer feedback remained correct.
- [x] iPhone discovery/connection was intentionally unavailable because this build injects BLE initialization failure before advertising.
- [x] No BLE advertising or connection occurred, as required by the injected failure.
- [x] Cancellation and completion each appended exactly once as sequences `202` and `203`.
- [x] Production firmware was restored and smoke-tested in `20260814T191842Z-production`.

## Measurements

| Measurement | Value | How observed |
| --- | --- | --- |
| Idle free heap | not measured | |
| Connected free heap | not measured | |
| Transfer minimum free heap | not measured | |
| Negotiated ATT MTU/value bytes | not measured | |
| Logical response bytes/time | not measured | |
| Watchdog or reset count | 0 after the initial USB reset | no panic, watchdog, restart, or second boot banner in the captured monitor log |
| Whole-device USB current | not measured | |

## Notes

The `radio-failure-diagnostic` build deliberately skipped `BleRadio::start` and
logged `injected BLE initialization failure; continuing as a complete offline
timer`. Existing settings loaded and the journal reconstructed with bounds
`138..201` before the test.

The first physical session exercised start, pause, resume, and long-press
cancellation and committed exactly one append as sequence `202`. The second
session completed while the encoder was rotated slowly and rapidly in both
directions, sounded the completion buzzer, and committed exactly one append as
sequence `203`. Dismiss returned to READY. The full interaction occurred with
no BLE host, advertising, connection, or protocol processing and without a
reset or watchdog event.
