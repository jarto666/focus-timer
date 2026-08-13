# Repository workflows

- `check-device.sh`: Rust formatting, lint, and host tests; no Node.js needed.
- `build-firmware.sh`: default or feature-selected ESP32 firmware build.
- `flash-firmware.sh`: build, flash, and monitor a connected controller.
- `check-mobile.sh`: mobile/package checks once the pnpm workspace exists.
- `check-protocol.sh`: Rust/TypeScript protocol compatibility checks.
- `check-all.sh`: combined non-hardware validation.
