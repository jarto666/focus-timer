# BLE acceptance observation

- Run: `20260814T123630Z-ble-echo`
- Firmware mode: `ble-echo`
- Commit: `0c7ebc1146c1e9e8a4dcc3a7259c2b07e6363afd`
- Port: `/dev/cu.usbmodem101`

## Visible behavior

- [ ] OLED remained responsive.
- [ ] Encoder rotation and button gestures remained correct.
- [ ] Start, pause, resume, cancel, complete, and dismiss behaved correctly.
- [ ] Buzzer feedback remained correct.
- [x] iPhone discovery, connection, subscription, write, and notification recorded below.
- [x] Disconnect, advertising restart, rediscovery, and second connection recorded below.
- [ ] Journal/result observation recorded below.
- [ ] Production firmware was restored after diagnostics.

## Measurements

| Measurement | Value | How observed |
| --- | --- | --- |
| Idle free heap | not measured | |
| Connected free heap | not measured | |
| Transfer minimum free heap | not measured | |
| Negotiated ATT MTU/value bytes | 256 / 253 | NimBLE MTU update after both iPhone connections |
| Logical response bytes/time | 23-byte write and 23-byte notification | Two complete echo cycles in `monitor.log` |
| Watchdog or reset count | 0 unexpected resets | One explicit USB reset followed by both echo cycles |
| Whole-device USB current | not measured | |

## Notes

The physical iPhone running Muninn discovered the service twice. Each attempt
connected, negotiated ATT MTU 256, subscribed to notifications, wrote a 23-byte
Hello request, received the same 23 bytes from the echo characteristic, and
disconnected cleanly. The firmware restarted advertising after each disconnect;
the second connection proves foreground rediscovery after restart.

Muninn reported `Transport failed, unsupportedMessage: 1`, which is the expected
application result: message kind 1 is a Hello request and the deliberately
protocol-free echo image returns it unchanged instead of producing a Hello
response. This proves the GATT byte path without claiming production handshake
or journal synchronization. OLED, encoder, buzzer, journal, and production
restore belong to the following integrated acceptance run.
