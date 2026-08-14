# Repository workflows

- `check-device.sh`: Rust formatting, lint, and host tests; no Node.js needed.
- `build-firmware.sh`: default or feature-selected ESP32 firmware build.
- `flash-firmware.sh`: build, flash, and monitor a connected controller.
- `ble-acceptance.sh`: build an evidence-labelled production/BLE/journal image;
  optionally inspect, flash, and capture a serial transcript from exactly one
  explicitly selected native-USB controller.
- `check-mobile.sh`: mobile/package checks once the pnpm workspace exists.
- `check-protocol.sh`: required Rust/TypeScript/shared-fixture compatibility
  gate, including format, lint, type, canonical-byte, malformed-input, and
  recovery tests.
- `check-all.sh`: combined non-hardware validation; includes the protocol gate.
