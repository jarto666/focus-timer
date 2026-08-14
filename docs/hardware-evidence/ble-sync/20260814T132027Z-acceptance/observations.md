# BLE acceptance observation

- Run: `20260814T132027Z-acceptance`
- Firmware mode: `acceptance`
- Commit: `0c7ebc1146c1e9e8a4dcc3a7259c2b07e6363afd`
- Port: `/dev/cu.usbmodem101`

## Visible behavior

- [x] OLED remained responsive.
- [x] Encoder rotation and button gestures remained correct.
- [x] Start, pause, and paused cancellation behaved correctly; completion,
      resume, and dismissal were not exercised in this run.
- [x] Start buzzer feedback remained correct; completion feedback was not
      exercised in this run.
- [x] iPhone discovery/connection/synchronization observation recorded below.
- [x] Disconnect/reconnect and advertising restart observation recorded below.
- [x] Journal/result observation recorded below.
- [ ] Production firmware was restored after diagnostics.

## Measurements

| Measurement                    | Value                                               | How observed                                                                     |
| ------------------------------ | --------------------------------------------------- | -------------------------------------------------------------------------------- |
| Idle free heap                 | 137712 B                                            | ESP-IDF `MALLOC_CAP_8BIT` snapshot after OLED, NVS, and BLE init                 |
| Connected free heap            | 137636 B first connection; 137548 B after reconnect | ESP-IDF `MALLOC_CAP_8BIT` snapshots                                              |
| Transfer minimum free heap     | 135008 B                                            | Minimum observed after the eight-page catch-up                                   |
| Negotiated ATT MTU/value bytes | 185 B / 182 B                                       | Physical iPhone MTU update and ATT framing limit                                 |
| Logical response bytes/time    | 356 B / 3 frames / 10 ms per full page              | Firmware transfer instrumentation; final short page was 353 B / 3 frames / 10 ms |
| Watchdog or reset count        | 0 unexpected                                        | Clean boot and continuous serial monitor                                         |
| Whole-device USB current       | not measured                                        |                                                                                  |

## Notes

The resource-instrumented integrated image recovered all 64 retained records,
initialized OLED and BLE, and remained in the authoritative event loop. At the
`idle-ready` checkpoint, free and minimum 8-bit heap were both 137,712 B and
the main task still had a 30,016 B stack high-water reserve.

After the physical iPhone resumed Muninn, the remembered device reconnected,
negotiated ATT MTU 185, completed Hello, clock-anchor, and status exchanges,
then caught up sequences 132 through 195 as eight bounded pages. Every full
356-byte page used three notifications and completed in 10 ms. Minimum 8-bit
heap during that transfer was 135,008 B; the main-task stack reserve remained
30,016 B. No watchdog or reset occurred.

While the same integrated image remained active, physical encoder rotation
continued to update and persist preset selection. A short press started an
eight-second session, a second press paused it, and a long press cancelled it.
The journal emitted one and only one append (`sequence=196`) and OLED returned
to READY. On a later reconnect the iPhone requested only the new suffix and
the device returned that one record in a 97-byte response, demonstrating the
device-side exact-once cursor path without replaying the preceding 64 records.

One reconnect exposed a mobile orchestration defect: the app had captured its
wall-clock value before BLE connection and handshake, so firmware correctly
rejected the stale value as `UtcMovedBackwards`. The app then recovered by
disconnecting and reconnecting. The implementation was changed to sample wall
time immediately before `SetClockAnchor`; a regression test now asserts that
the clock provider is called after Hello. Physical confirmation of that fix is
kept pending until the updated Metro bundle is loaded.
