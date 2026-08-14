# BLE acceptance observation

- Run: `20260814T133508Z-journal-corrupt`
- Firmware mode: `journal-corrupt`
- Commit: `0c7ebc1146c1e9e8a4dcc3a7259c2b07e6363afd`
- Port: `/dev/cu.usbmodem101`

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

| Measurement                    | Value        | How observed |
| ------------------------------ | ------------ | ------------ |
| Idle free heap                 | not measured |              |
| Connected free heap            | not measured |              |
| Transfer minimum free heap     | not measured |              |
| Negotiated ATT MTU/value bytes | not measured |              |
| Logical response bytes/time    | not measured |              |
| Watchdog or reset count        | not measured |              |
| Whole-device USB current       | not measured |              |

## Notes

This attempt built and preserved the exact diagnostic artifact, but the
sandboxed runner was denied access to `/dev/cu.usbmodem101` during board
inspection. Nothing was flashed. The immediately following
`20260814T133525Z-journal-corrupt` run repeated the same closed diagnostic mode
with USB access and is the physical evidence.
