# BLE acceptance observation

- Run: `20260814T130748Z-journal-fill`
- Firmware mode: `journal-fill`
- Commit: `0c7ebc1146c1e9e8a4dcc3a7259c2b07e6363afd`
- Port: `/dev/cu.usbmodem101`

## Visible behavior

- [ ] OLED remained responsive.
- [ ] Encoder rotation and button gestures remained correct.
- [ ] Start, pause, resume, cancel, complete, and dismiss behaved correctly.
- [ ] Buzzer feedback remained correct.
- [ ] iPhone discovery/connection/synchronization observation recorded below.
- [ ] Disconnect/reconnect and advertising restart observation recorded below.
- [x] Journal/result observation recorded below.
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

The isolated diagnostic appended 65 records on each boot without opening the
`focus_timer` settings namespace. After two boots it reported
`bounds=(67,130)`, `high_water=130`, `stale_cursor_gap=true`, and an eight-record
first page. The following integrated boot recovered the same identity/epoch and
the last 64 records as `Healthy`, proving physical eviction and reboot
reconstruction. These are diagnostic records, not user-operated sessions.
