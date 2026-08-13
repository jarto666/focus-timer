# Focus Timer

Focus Timer is an offline-first physical timer and its companion software. The
validated USB-powered ESP32-C3 MVP uses an EC11 encoder, SSD1306 OLED, and 3 V
active buzzer. Bluetooth synchronization and the iPhone companion are being
added without making the phone part of timer correctness.

## Product layout

- `device/`: Rust workspace for pure domain behavior and ESP32 firmware.
- `apps/`: product applications; the iPhone-first Expo app lives in
  `apps/mobile/` once initialized.
- `packages/`: reusable TypeScript protocol, device-client, and mock-device
  packages.
- `protocol/`: transport-independent schema and cross-language golden fixtures.
- `scripts/`: root-invocable scoped and combined checks.
- `docs/`: hardware, wiring, development, and acceptance evidence.
- `openspec/`: product changes and capability specifications.

The Rust workspace currently contains:

- `device/crates/focus-core`: `no_std` preset, session, input, and settings
  behavior.
- `device/crates/focus-firmware`: `std + ESP-IDF` executable and hardware
  adapters.

## Prerequisites and checks

Device-only work requires Rust plus the documented ESP toolchain, and does not
require Node.js:

```sh
./scripts/check-device.sh
./scripts/build-firmware.sh
```

Mobile and shared TypeScript work use Node.js 24.12.0, pnpm 10.32.1, Expo SDK
57, React 19.2.3, and React Native 0.86.2. They do not require a connected
ESP32 when using the mock device:

```sh
corepack enable
pnpm install --frozen-lockfile
./scripts/check-mobile.sh
./scripts/check-protocol.sh
./scripts/check-all.sh
```

See [docs/development.md](docs/development.md) for development and flash
commands, [docs/hardware.md](docs/hardware.md) for prototype evidence,
[docs/wiring.md](docs/wiring.md) to reproduce or transfer the circuit, and
[docs/acceptance.md](docs/acceptance.md) for the offline-MVP capability audit.
The oversized external WS2812 ring remains disconnected and deferred.
