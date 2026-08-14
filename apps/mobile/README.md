# Focus Timer mobile

The iPhone companion is an Expo Router application. It starts with a
deterministic mock backend so UI and synchronization work can continue without
the timer or native BLE tooling.

## Run the app

Install the pinned workspace from the repository root:

```sh
corepack enable
pnpm install --frozen-lockfile
pnpm --filter @focus-timer/mobile start
```

The default backend is `mock`. It can also be selected explicitly:

```sh
EXPO_PUBLIC_DEVICE_BACKEND=mock pnpm --filter @focus-timer/mobile start
```

Run the same development build against the physical foreground BLE adapter:

```sh
EXPO_PUBLIC_DEVICE_BACKEND=ble \
  pnpm --filter @focus-timer/mobile exec expo start --dev-client --lan --clear
```

The adapter remains at the application composition boundary; transport checks
do not leak into screens.

## Architecture boundary

- `src/app` contains Expo Router screens and presentation only.
- `src/application/runtime-provider.tsx` is the composition boundary that
  selects the configured backend.
- `packages/device-client` exposes the transport-neutral device model.
- `packages/mock-device` and `src/ble` implement the same request contract.

Screens must not import BLE libraries or branch on the selected backend.

## Physical iPhone build

The app uses an Expo Development Build, not Expo Go, because
`react-native-ble-plx` 3.5.1 requires native code. The native shell with BLE has
been compiled, signed, and installed on a physical iPhone with Xcode 26.6,
CocoaPods 1.17.0, an iOS 16.4 deployment target, and `expo-dev-client` 57.0.11.
Installation is also proven over the local network with the USB cable
disconnected. See
[`docs/mobile-development.md`](../../docs/mobile-development.md) for the pinned
toolchain, build commands, keychain-signing workaround, and wireless workflow.
