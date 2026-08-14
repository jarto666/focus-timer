# Mobile development

The iPhone companion is an Expo Development Build backed by the same
transport-neutral packages used by the deterministic mock. Expo Go is not a
target because the physical transport will require a native BLE dependency.

## Pinned JavaScript baseline

| Component    | Version |
| ------------ | ------- |
| Node.js      | 24.12.0 |
| pnpm         | 10.32.1 |
| Expo SDK     | 57.0.12 |
| React        | 19.2.3  |
| React Native | 0.86.2  |
| TypeScript   | 6.0.3   |
| ESLint       | 9.39.5  |
| Prettier     | 3.9.6   |
| Vitest       | 4.1.10  |

## Proven native baseline

| Component       | Proven version |
| --------------- | -------------- |
| Xcode           | 26.6 (17F113)  |
| CocoaPods       | 1.17.0         |
| iOS target      | 16.4           |
| expo-dev-client | 57.0.11        |

The baseline was compiled, signed, installed, and launched on a physical
iPhone 13 Pro running iOS 26.6. A second install and launch was proven with the
USB cable disconnected; CoreDevice reported `transportType: localNetwork`.

Expo SDK 57 officially requires Node.js 22.13 or newer, React Native 0.86,
React 19.2.3, iOS 16.4 or newer, and Xcode 26.4 or newer. This repository pins
the exact versions proven by local workspace checks rather than following npm
`latest` during installs.

## Install and check

From the repository root:

```sh
corepack enable
pnpm install --frozen-lockfile
pnpm check
./scripts/check-mobile.sh
./scripts/check-protocol.sh
```

The root TypeScript gate runs Prettier, Expo-aware ESLint with warnings denied,
strict TypeScript checks, and Vitest. `./scripts/check-device.sh` remains
independent and does not require Node.js or the pnpm installation.

## Workspace boundaries

- `apps/mobile` owns Expo Router screens, native adapters, and phone-local
  persistence.
- `packages/device-protocol` owns logical messages and canonical bytes.
- `packages/device-client` owns connection state and typed request/sync APIs.
- `packages/mock-device` owns deterministic hardware-free scenarios.

The screen layer must depend on the device-client contract, never directly on
BLE objects. The mock and physical transports are selected at the application
composition boundary.

## Backend selection

The application defaults to the deterministic mock:

```sh
pnpm --filter @focus-timer/mobile start
```

The same choice can be made explicitly with
`EXPO_PUBLIC_DEVICE_BACKEND=mock`. The future physical build will use
`EXPO_PUBLIC_DEVICE_BACKEND=ble`; its adapter will be added at the composition
boundary. Screens never read the environment variable or import a native BLE
object.

## Physical iPhone Development Build

The machine's global `xcode-select` may still point at Apple Command Line Tools,
so the proven commands select full Xcode explicitly without changing the global
developer directory:

```sh
env DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  COCOAPODS_PARALLEL_CODE_SIGN=false \
  pnpm --filter @focus-timer/mobile exec expo run:ios \
  --device <xcode-device-udid> --no-bundler
```

Disabling parallel CocoaPods code signing is intentional. It prevents multiple
pods from simultaneously requesting access to the Apple Development key in the
login keychain.

Run Metro on the LAN for the installed Development Build:

```sh
env DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  pnpm --filter @focus-timer/mobile exec expo start \
  --dev-client --lan --clear
```

For wireless development, pair the iPhone with Xcode once over USB, enable
Developer Mode, allow network connection in Xcode, and keep the Mac and iPhone
on the same local network. With the cable disconnected, verify the transport:

```sh
env DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  xcrun devicectl device info details --device <coredevice-identifier>
```

The output must contain `transportType: localNetwork`. The signed app can then
be installed and launched over that connection:

```sh
env DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  xcrun devicectl device install app \
  --device <coredevice-identifier> <path-to-signed-app>

env DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  xcrun devicectl device process launch --terminate-existing \
  --device <coredevice-identifier> com.jarto666.focustimer
```

This proves the native shell and LAN development loop. The BLE dependency is a
separate milestone and will be pinned only after its native iPhone build is
proven.
