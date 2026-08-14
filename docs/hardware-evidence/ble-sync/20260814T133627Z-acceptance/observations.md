# BLE acceptance observation

- Run: `20260814T133627Z-acceptance`
- Firmware mode: `acceptance`
- Commit: `0c7ebc1146c1e9e8a4dcc3a7259c2b07e6363afd`
- Port: `/dev/cu.usbmodem101`

## Visible behavior

- [x] OLED initialized and rendered READY after journal recovery.
- [x] Encoder rotation and button gestures remained correct.
- [x] Start, running cancellation, completion, and dismissal behaved correctly after degraded recovery.
- [x] Buzzer start and completion feedback remained correct.
- [x] iPhone discovery/connection/synchronization observation recorded below.
- [x] Disconnect/reconnect and advertising restart observation recorded below.
- [x] Journal/result observation recorded below.
- [ ] Production firmware was restored after diagnostics.

## Measurements

| Measurement                    | Value        | How observed                                                      |
| ------------------------------ | ------------ | ----------------------------------------------------------------- |
| Idle free heap                 | 137712 B     | ESP-IDF snapshot after degraded recovery, OLED, NVS, and BLE init |
| Connected free heap            | 137636 B     | ESP-IDF snapshot after the iPhone connected                       |
| Transfer minimum free heap     | 135268 B     | ESP-IDF minimum after Hello, clock, status, and page transfers    |
| Negotiated ATT MTU/value bytes | 185 / 182 B  | NimBLE MTU update and bounded GATT transport                      |
| Logical response bytes/time    | 133 B / 2 ms | Largest response in the two-record suffix synchronization        |
| Watchdog or reset count        | 0 unexpected | Clean integrated boot after one intentional USB reset             |
| Whole-device USB current       | not measured |                                                                   |

## Notes

After both redundant metadata blobs and slot `s00` were corrupted, production
initialization isolated exactly one invalid slot and reconstructed the unique
remaining journal generation. It retained stable device identity and epoch,
reported bounds 133 through 196 and `health=Degraded`, and loaded the existing
Quick Sprint preset from the separate settings namespace. The authoritative
loop then initialized OLED and BLE, rendered `READY / Quick Sprint / 00:08`,
and advertised normally with no reset or watchdog event.

After recovery, physical encoder rotation selected Deep Work and then Focus.
The button started Focus, emitted the start buzzer effect, and rendered
`FOCUS / 00:08`. Rotation while running was safely ignored and a long press
cancelled the session. The firmware appended exactly one journal record at
sequence 197 and returned to `READY / Focus` without a reset or watchdog event.

The physical iPhone then reloaded the BLE application over the explicitly
approved Expo LAN development connection. It reconnected to the remembered
device, negotiated ATT MTU 185, and completed Hello, clock-anchor, status, and
one-record suffix-page requests.

After the app was terminated and BLE reported a clean remote-user disconnect,
the physical timer ran a complete eight-second Focus session without the app.
It rendered every second, emitted completion feedback, appended exactly one
completed outcome at sequence 199, displayed COMPLETE, and dismissed back to
READY on a short press. No BLE request or connection was active during this
session.

A later reconnect reproduced a valid real-world edge case: crystal drift made
the phone's refreshed UTC fractionally older than the device's extrapolated
anchor, so firmware correctly rejected it as `UtcMovedBackwards`. The updated
mobile sync treated that optional clock refresh as non-fatal, then continued
with status and a 133-byte suffix page containing sequences 198 and 199. The
user confirmed the new completed Focus record in Muninn's SQLite-backed History
screen, unplugged the board, observed the offline state, and confirmed that the
same history remained available. Across the reconnect and transfer, the device
retained at least 135268 B free 8-bit heap and 29648 B main-stack reserve.
