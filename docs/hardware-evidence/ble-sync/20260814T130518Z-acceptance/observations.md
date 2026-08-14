# BLE acceptance observation

- Run: `20260814T130518Z-acceptance`
- Firmware mode: `acceptance`
- Commit: `0c7ebc1146c1e9e8a4dcc3a7259c2b07e6363afd`
- Port: `/dev/cu.usbmodem101`

## Visible behavior

- [x] OLED remained responsive.
- [ ] Encoder rotation and button gestures remained correct.
- [ ] Start, pause, resume, cancel, complete, and dismiss behaved correctly.
- [ ] Buzzer feedback remained correct.
- [x] iPhone discovery/connection/synchronization observation recorded below.
- [ ] Disconnect/reconnect and advertising restart observation recorded below.
- [ ] Journal/result observation recorded below.
- [ ] Production firmware was restored after diagnostics.

## Measurements

| Measurement | Value | How observed |
| --- | --- | --- |
| Idle free heap | not measured | |
| Connected free heap | not measured | |
| Transfer minimum free heap | not measured | |
| Negotiated ATT MTU/value bytes | 185 / 182 | Controller connection and MTU logs |
| Logical response bytes/time | Hello 57 B; clock 25 B; status 67 B | Typed request/response logs |
| Watchdog or reset count | 0 unexpected | Clean boot and continuous serial monitor |
| Whole-device USB current | not measured | |

## Notes

Muninn connected over physical iPhone BLE and completed the production
handshake (`1 -> 2`), volatile clock anchor (`7 -> 8`), and immutable status
read (`3 -> 4`). Every response was received after a pre-write subscription;
the subscription was then removed by the one-request mobile transport. This
also proves the earlier `unsupportedMessage: 1` was specific to the echo image,
not the production protocol.
