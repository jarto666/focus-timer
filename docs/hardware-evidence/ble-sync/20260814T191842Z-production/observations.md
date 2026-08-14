# BLE acceptance observation

- Run: `20260814T191842Z-production`
- Firmware mode: `production`
- Commit: `0c7ebc1146c1e9e8a4dcc3a7259c2b07e6363afd`
- Port: `/dev/cu.usbmodem101`

## Visible behavior

- [x] OLED initialized in normal production mode and displayed the persisted `Focus 50:00` preset.
- [x] Encoder rotation selected Pomodoro and returned to Focus; both selections rendered immediately.
- [x] Full production timer gestures were already covered by the immediately preceding physical runs; this restoration smoke test verified selection and persistence.
- [x] Production buzzer behavior was covered before restoration and its GPIO initialized normally on this boot.
- [x] The iPhone automatically reconnected, completed Hello/clock/status, and requested the journal suffix containing diagnostic outcomes `202..203`.
- [x] BLE advertising started, the remembered iPhone reconnected, and MTU 185 was negotiated.
- [x] NVS reconstructed journal bounds `140..203`, preserving both radio-failure outcomes, and loaded settings successfully.
- [x] This is the restored default production firmware; no diagnostic warning or shortened preset was present.

## Measurements

| Measurement | Value | How observed |
| --- | --- | --- |
| Negotiated ATT MTU/value bytes | 185 / 182 bytes | physical reconnect after production restoration |
| Logical response bytes/time | 143 bytes / 2 ms | journal suffix response containing sequences `202..203` |
| Watchdog or reset count | 0 after the intentional USB reset | no panic, watchdog, restart, or second boot banner in the captured monitor log |
| Whole-device USB current | not measured | |

## Notes

The default feature set was rebuilt and flashed after all fault diagnostics.
Boot restored the same stable device identity and epoch, loaded the settings
record, reconstructed the journal through sequence `203`, initialized the BLE
host, and advertised `FocusTimer`. The OLED showed the real persisted
`Focus 50:00` duration rather than the eight-second diagnostic duration.

The normal iPhone build automatically reconnected, negotiated MTU 185, completed
Hello, clock-anchor, and status requests, and fetched a 143-byte journal suffix.
Encoder selection changes rendered and were saved back to NVS. This confirms
that the diagnostic builds did not damage settings, journal state, or the
production BLE path.

With this same production build connected, Bluetooth was then disabled using
the system Settings switch. The initial implementation exposed the link-loss
fallback as `Unpaired`, which was incorrect because the stable remembered
device and local history still existed. The transport contract was extended
with live availability events and the runtime now presents `Bluetooth is off`
while preserving local history. The user repeated the physical toggle against
the updated iPhone bundle and confirmed the corrected state works.
