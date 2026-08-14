# BLE acceptance observation

- Run: `20260814T182346Z-ble-faults`
- Firmware mode: `ble-faults`
- Commit: `0c7ebc1146c1e9e8a4dcc3a7259c2b07e6363afd`
- Port: `/dev/cu.usbmodem101`

## Visible behavior

- [x] OLED remained responsive after the deliberately interrupted multi-fragment response and counted from FOCUS to COMPLETE and back to READY.
- [x] Slow and fast encoder rotation in both directions and short presses remained responsive after the BLE lifecycle fault.
- [x] Start, completion, and dismiss behaved correctly; pause/resume/cancel were covered by adjacent physical acceptance runs.
- [x] Start and completion buzzer feedback remained correct.
- [x] The iPhone connected, negotiated MTU 185, completed the corrupt/incompatible/unknown/malformed/duplicate matrix, and recovered with a valid status request.
- [x] The iPhone disconnected after the first fragment of a three-fragment page response; firmware dropped the stale response and restarted advertising.
- [x] Completion appended exactly one journal record, sequence `201`.
- [x] Production firmware was restored and smoke-tested in `20260814T191842Z-production`.

## Measurements

| Measurement | Value | How observed |
| --- | --- | --- |
| Idle free heap | 137,848 bytes | `phase=idle-ready` resource snapshot |
| Connected free heap | 137,772 bytes | `phase=connected` resource snapshot |
| Transfer minimum free heap | 135,380 bytes | minimum observed during the duplicate multi-fragment responses |
| Negotiated ATT MTU/value bytes | 185 / 182 bytes | NimBLE MTU update and three-frame 360-byte responses |
| Logical response bytes/time | 360 bytes / 196 ms | acceptance-only 100 ms fragment spacing made the disconnect point reproducible |
| Watchdog or reset count | 0 after the initial USB boot | no panic, watchdog, restart, or second boot banner in the captured monitor log |
| Whole-device USB current | not measured | |

## Notes

This run used the `ble-fault-diagnostic` feature, which changes only diagnostic
timing by spacing response fragments 100 ms apart. The iPhone disconnected at
22,318 ms after the first notification of a 360-byte page response. Firmware
incremented the connection generation, reset the protocol session, restarted
advertising, and logged `BLE logical response dropped after connection lifecycle
changed` instead of sending the remaining fragments to the stale connection.

After that failure the standalone timer started normally, accepted sustained
slow and fast rotation in both directions, refreshed the OLED every second,
completed, sounded the buzzer, committed exactly one append as sequence `201`,
and dismissed back to READY. No reset or watchdog event occurred.
