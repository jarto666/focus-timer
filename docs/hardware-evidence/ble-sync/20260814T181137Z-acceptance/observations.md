# BLE acceptance observation

- Run: `20260814T181137Z-acceptance`
- Firmware mode: `acceptance`
- Commit: `0c7ebc1146c1e9e8a4dcc3a7259c2b07e6363afd`
- Port: `/dev/cu.usbmodem101`

## Visible behavior

- [x] OLED remained responsive before, during, and after the protocol fault matrix.
- [x] Slow and fast encoder rotation in both directions and short presses remained responsive.
- [x] Start, pause, resume, complete, and dismiss behaved correctly; cancellation was covered by the preceding physical acceptance run.
- [x] Start and completion buzzer feedback remained correct.
- [x] The iPhone connected, negotiated MTU 185, ran the native protocol matrix, and recovered with a valid status request after malformed input.
- [x] Disconnect restarted advertising and did not reset or stall the standalone timer.
- [x] Completion appended exactly one journal record, sequence `200`.
- [x] Production firmware was restored and smoke-tested in `20260814T191842Z-production`.

## Measurements

| Measurement | Value | How observed |
| --- | --- | --- |
| Idle free heap | 137,712 bytes | `phase=idle-ready` resource snapshot |
| Connected free heap | 137,636 bytes | `phase=connected` resource snapshot |
| Transfer minimum free heap | 135,004 bytes | minimum observed during the 360-byte page responses |
| Negotiated ATT MTU/value bytes | 185 / 182 bytes | NimBLE MTU update and three-frame 360-byte responses |
| Logical response bytes/time | 360 bytes / 10 ms | two identical duplicate page responses |
| Watchdog or reset count | 0 after the initial USB boot | no panic, watchdog, restart, or second boot banner in the captured monitor log |
| Whole-device USB current | not measured | |

## Notes

The physical fault-matrix build exercised a corrupt GATT frame (`FrameTooShort`),
two byte-identical duplicate page requests, an incompatible protocol-major
handshake, an unknown command, and a malformed one-byte logical payload. The
firmware rejected each invalid input and subsequently served a valid status
request. The first disconnect attempt occurred after the three-frame response
had already completed, so the dedicated slowed-fragment run below was added to
prove a real disconnect between notifications.

The timer was paused while the iPhone exercised the protocol matrix, then
resumed and completed while the encoder was rotated rapidly in both directions.
The OLED counted down, completion feedback sounded, exactly one journal append
was committed as sequence `200`, and dismiss returned to READY.
