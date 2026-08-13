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

## Current native-tooling blocker

The JavaScript workspace is reproducible, but the machine currently selects
Apple Command Line Tools rather than a full Xcode installation, `simctl` is
unavailable, and CocoaPods is not installed. A physical iPhone development
build cannot be claimed until full Xcode 26.4 or newer is installed and selected
and the generated native project builds successfully. The exact native and BLE
dependency set will be pinned only after that proof.
