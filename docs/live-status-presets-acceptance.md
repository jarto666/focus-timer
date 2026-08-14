# Live status and custom presets acceptance

This checkpoint covers OpenSpec change `add-live-status-and-custom-presets`.
The implementation is complete enough for the first physical product pass;
fault-injection and long stress runs remain deliberately separate from the MVP.

## Automated evidence

- `./scripts/check-all.sh`: pass.
- Strict OpenSpec validation: pass.
- ESP32-C3 production target build: pass.
- Signed iOS device build: pass.
- Muninn development app installation on the paired iPhone: pass.
- `git diff --check`: pass.

The automated suites currently contain 27 protocol TypeScript tests, 15 device
client tests, 25 deterministic mock tests, 40 mobile tests, and the Rust core,
protocol, sync, firmware adapter, persistence, presentation, and compatibility
suites executed by `check-all`.

## Flashed production checkpoint

- Board: ESP32-C3 revision v0.4, 4 MB flash.
- Firmware image: 1,134,640 bytes of a 4,128,768-byte application partition
  (27.48%).
- Boot smoke test: OLED initialized, BLE advertising started, encoder/buzzer
  GPIO initialized, and the normal `READY / Focus / 50:00` screen rendered.
- Firmware mode: production; no acceptance or fault diagnostic feature enabled.

## User verification pass

Run these in order with Muninn open and the timer idle:

1. Rotate the encoder through several built-ins. The phone must update the
   selected preset without reconnecting.
2. Start, pause, resume, and cancel a session physically. The phone must follow
   each state and never expose remote lifecycle controls.
3. Start a session and watch the phone for at least ten seconds. Its display may
   interpolate, but each device snapshot must correct it without visible jumps
   larger than one second.
4. Open **Ritual catalog**, add one custom preset, and send it. The OLED must show
   the phone-requested update; short-press the encoder within 15 seconds.
5. Confirm that Muninn reports synchronization, the custom preset appears after
   the built-ins on the physical timer, and it starts with the phone absent.
6. Edit or delete that preset, send again, and long-press once to verify explicit
   rejection. Retry and short-press to commit.
7. Power-cycle the timer and verify the committed custom catalog survives.
8. Disconnect Bluetooth during a running session. Muninn must freeze the last
   authoritative snapshot as stale; reconnecting must converge to the device.

Do not mark the physical OpenSpec tasks complete until these observations have
actually been made. Corruption injection, notification loss/reordering, and
extended encoder/radio stress remain the later fault/stress pass.
