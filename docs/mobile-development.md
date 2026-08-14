# Mobile development

Muninn is an Expo Development Build backed by one transport-neutral client.
The same screens and SQLite sync path run against either the deterministic mock
or the physical BLE adapter. Expo Go is not supported because
`react-native-ble-plx` contains native code.

## Pinned baseline

| Component            | Version       |
| -------------------- | ------------- |
| Node.js              | 24.12.0       |
| pnpm                 | 10.32.1       |
| Expo SDK             | 57.0.12       |
| React                | 19.2.3        |
| React Native         | 0.86.2        |
| TypeScript           | 6.0.3         |
| react-native-ble-plx | 3.5.1         |
| expo-sqlite          | 57.0.1        |
| expo-dev-client      | 57.0.11       |
| Xcode                | 26.6 (17F113) |
| CocoaPods            | 1.17.0        |
| iOS deployment       | 16.4 or newer |

The native graph was compiled and signed, then installed on an iPhone 13 Pro
running iOS 26.6 over both USB and `localNetwork` CoreDevice transport.

## Install and check

From the repository root:

```sh
corepack enable
pnpm install --frozen-lockfile
./scripts/check-mobile.sh
./scripts/check-protocol.sh
```

The mobile gate runs Prettier, Expo-aware ESLint with warnings denied, strict
TypeScript checks, and Vitest. Device-only work remains independent of Node.js.

## Backend selection

Mock mode is the default and requires neither Xcode nor ESP hardware:

```sh
EXPO_PUBLIC_DEVICE_BACKEND=mock \
  pnpm --filter @focus-timer/mobile start
```

Physical mode uses the same application build and screen models:

```sh
EXPO_PUBLIC_DEVICE_BACKEND=ble \
  pnpm --filter @focus-timer/mobile exec expo start --dev-client --lan --clear
```

Only `src/application/runtime-provider.tsx` selects the adapter. Screens do not
import `react-native-ble-plx`, inspect the environment variable, or fork their
models for mock versus hardware.

## Native BLE configuration

`apps/mobile/app.json` pins the BLE config plugin with foreground-only behavior
and the iOS Bluetooth usage text. No background mode is enabled: correctness
comes from the durable timer journal and foreground catch-up, not continuous
background execution.

After changing a native dependency or app plugin, regenerate the ignored iOS
project and install pods:

```sh
env DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  pnpm --filter @focus-timer/mobile exec expo prebuild \
  --platform ios --clean --no-install

env DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  pod install --project-directory=apps/mobile/ios
```

Verify the generated `Info.plist` contains
`NSBluetoothAlwaysUsageDescription` and does not contain `UIBackgroundModes`.
The generated project uses scheme `Muninn`, bundle identifier
`com.jarto666.focustimer`, and the `appleTeamId` committed in `app.json`.

Build a signed development app without parallel pod signing:

```sh
cd apps/mobile/ios
env DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  COCOAPODS_PARALLEL_CODE_SIGN=false \
  xcodebuild -workspace Muninn.xcworkspace -scheme Muninn \
  -configuration Debug -destination generic/platform=iOS \
  -derivedDataPath /private/tmp/muninn-derived \
  -allowProvisioningUpdates build
```

`COCOAPODS_PARALLEL_CODE_SIGN=false` prevents many simultaneous login-keychain
prompts. If macOS asks once, enter the login password and choose **Always
Allow** only for the expected Apple Development key.

## Install and run on the iPhone

Pair the phone with Xcode once over USB, enable Developer Mode and network
connection, then keep the Mac and phone on the same LAN. Find the CoreDevice
identifier and confirm wireless transport:

```sh
env DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  xcrun devicectl list devices

env DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  xcrun devicectl device info details --device <coredevice-id>
```

The details should contain `transportType: localNetwork`. Install and launch:

```sh
env DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  xcrun devicectl device install app --device <coredevice-id> \
  /private/tmp/muninn-derived/Build/Products/Debug-iphoneos/Muninn.app

env DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  xcrun devicectl device process launch --terminate-existing \
  --device <coredevice-id> com.jarto666.focustimer
```

iOS refuses remote launch while the phone is locked. Unlock it and retry once;
this is not a build or installation failure. Keep exactly one Metro process on
port 8081 so the development client does not attach to the wrong backend.

## Foreground synchronization behavior

1. The app checks Bluetooth state and scans only for the Focus Timer service.
2. It connects to the selected iOS peripheral, discovers the required command
   and response characteristics, and subscribes before the first write.
3. The protocol handshake supplies the stable product identity. The iOS
   peripheral identifier is retained only as a reconnect hint.
4. A foreground cold launch attempts the remembered peripheral once. Failure
   falls back to the explicit reconnect/search UI without a retry loop.
5. The app sends a volatile clock anchor, reads immutable timer status, and
   fetches journal pages until current. It samples wall time after connection
   and Hello, immediately before `SetClockAnchor`; sampling before BLE setup can
   make a reconnect anchor stale enough for the firmware to reject it as time
   moving backwards.
6. Each page and cursor commit in one SQLite transaction. A disconnect or app
   termination retries from the last committed cursor.
7. The app keeps local history readable while Bluetooth and the timer are
   unavailable. A later foreground launch attempts the remembered link once
   and still offers explicit reconnection if that attempt fails.

The BLE transport permits one request at a time, correlates both BLE transfer
and logical request identifiers, applies explicit timeouts/cancellation, and
resets bounded reassembly after malformed or partial input.

## Durable local history

`muninn.sqlite` stores known-device metadata, journal epochs/cursors, retention
gaps, and session records keyed by `(device_id, journal_epoch, sequence)`.
Nullable UTC timestamps stay nullable; the app never fabricates time or cloud
recovery. Mock and BLE sessions enter this same repository boundary.

## Troubleshooting

- **Bluetooth disabled:** Muninn shows the unavailable state. Enable Bluetooth
  in iOS and search again.
- **Permission denied:** use the app's **Open Bluetooth settings** action, grant
  access, then return and retry.
- **No timer found:** confirm the firmware is powered, advertising, and not
  connected to another central. The scan intentionally filters by service UUID.
- **Connects but cannot handshake:** verify the service/characteristic UUIDs and
  protocol major version against `protocol/schema.md`.
- **History sync stops:** local rows remain committed. Reconnect; synchronization
  resumes from the SQLite cursor instead of replaying UI memory.
- **Metro opens mock data:** stop the existing Metro process and restart it with
  `EXPO_PUBLIC_DEVICE_BACKEND=ble` before launching the development build.

## MVP security boundary

The development BLE service is personal-bench software, not production
ownership pairing. It exposes no remote timer control, preset mutation, Wi-Fi
credentials, account/cloud identifier, OTA path, or journal-payload logging.
The bounded clock anchor cannot affect monotonic session accounting. Do not
distribute this build until a separate ownership and pairing change is designed.
