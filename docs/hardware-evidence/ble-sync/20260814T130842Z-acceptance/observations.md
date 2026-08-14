# BLE acceptance observation

- Run: `20260814T130842Z-acceptance`
- Firmware mode: `acceptance`
- Commit: `0c7ebc1146c1e9e8a4dcc3a7259c2b07e6363afd`
- Port: `/dev/cu.usbmodem101`

## Visible behavior

- [x] OLED remained responsive.
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
| Watchdog or reset count | 0 unexpected | Clean boot and continuous serial monitor |
| Whole-device USB current | not measured | |

## Notes

The production runtime physically recovered a full evicted journal as
`bounds=(132,195)`, `health=Healthy`, with stable device identity and epoch.
The sequence advanced beyond the diagnostic's explicit `(67,130)` observation
because exiting the fill monitor reset that still-installed diagnostic once
more before the integrated image was restored. OLED and BLE advertising then
started normally.
