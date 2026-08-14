# BLE acceptance observation

- Run: `20260814T133525Z-journal-corrupt`
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
- [x] Journal/result observation recorded below.
- [x] Integrated acceptance firmware was restored after diagnostics; final
      production restore remains part of the end-of-change handoff.

## Measurements

| Measurement                    | Value        | How observed                                        |
| ------------------------------ | ------------ | --------------------------------------------------- |
| Idle free heap                 | not measured |                                                     |
| Connected free heap            | not measured |                                                     |
| Transfer minimum free heap     | not measured |                                                     |
| Negotiated ATT MTU/value bytes | not measured |                                                     |
| Logical response bytes/time    | not measured |                                                     |
| Watchdog or reset count        | 0 unexpected | Clean diagnostic boot and one intentional USB reset |
| Whole-device USB current       | not measured |                                                     |

## Notes

The focused alternative image opened only `focus_sync`, replaced `meta_a`,
`meta_b`, and `s00` with invalid blobs, logged the exact closed mutation, and
then remained idle. The following integrated run is the recovery half of this
experiment. The `focus_timer` settings namespace was never opened by this
diagnostic.
