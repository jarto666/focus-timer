# BLE acceptance observation

- Run: `20260814T130153Z-acceptance`
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
| Watchdog or reset count | 0 unexpected | Clean boot and continuous serial monitor |
| Whole-device USB current | not measured | |

## Notes

First clean integrated boot after compact journal reconstruction. The OLED
initialized and rendered `READY / Quick Sprint / 00:08`; the existing stable
identity and empty epoch reconstructed as `Healthy`; NimBLE advertised without
a reset or watchdog event. No physical input gestures were performed in this
run.
