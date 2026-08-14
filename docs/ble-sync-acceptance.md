# BLE sync acceptance ledger

This ledger audits every scenario introduced by OpenSpec change
`add-ble-device-sync`. `Automated` means the behavior is covered by a committed
host/mobile test or strict build. `Native` means the generated iOS graph was
signed and installed on the physical iPhone. `Bench follow-up` is deliberately
not a pass: it names the remaining physical evidence required before archive.

## Product workspace

| Scenario                                                      | Evidence or explicit follow-up                                                                      |
| ------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Developer locates product areas                               | Automated: root layout and links in `README.md`; root scoped check scripts.                         |
| New application can be added without moving device code again | Automated: independent `device/`, `apps/`, `packages/`, and `protocol/` workspaces.                 |
| Fresh checkout validation                                     | Automated: frozen pnpm install and root check ledger below.                                         |
| Firmware flashing remains reproducible                        | Automated build command in `docs/development.md`; bench flashing remains in the physical checklist. |
| Host checkpoint after migration                               | Automated: `check-device`, `check-protocol`, and `check-mobile` gates.                              |
| On-device smoke test after migration                          | Existing pre-BLE evidence in `docs/acceptance.md`; BLE-integrated repeat is a bench follow-up.      |
| Device-only developer                                         | Automated: `scripts/check-device.sh` does not invoke Node.js.                                       |
| Mobile-only development                                       | Automated: mock backend and TypeScript gates need no ESP hardware.                                  |

## Device sync protocol

| Scenario                              | Evidence or explicit follow-up                                                             |
| ------------------------------------- | ------------------------------------------------------------------------------------------ |
| Compatible handshake                  | Rust/TypeScript fixture and codec tests plus `device-client.test.ts`.                      |
| Incompatible major version            | Rust protocol-session and TypeScript client/mock tests.                                    |
| Unknown request type                  | Rust/TypeScript malformed tests and firmware protocol-session recovery test.               |
| Golden request fixture                | Cross-language fixture gates in both protocol packages.                                    |
| Historical fixture remains readable   | Cross-language historical fixture decoding tests.                                          |
| Oversized message                     | Rust and TypeScript codec/transport bound tests.                                           |
| Invalid payload                       | Truncation, duplicate-key, non-canonical, checksum, and recovery tests in both languages.  |
| Fragmented BLE transfer               | Rust/TypeScript multi-frame tests and native-adapter bidirectional fragmentation test.     |
| Retransmitted read request            | Repeatable journal page and duplicate-page idempotence tests.                              |
| Current status read                   | Immutable status projection and typed client API tests.                                    |
| Clock anchor update                   | Host clock tests, protocol codec tests, and foreground sync flow.                          |
| Remote-control request is unavailable | Command-surface audit: only hello, status, page, and volatile clock-anchor requests exist. |

## Session journal sync

| Scenario                                             | Evidence or explicit follow-up                                                                         |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Completed session is recorded once                   | `focus-core` lifecycle and firmware journal-adapter tests; exact physical append is a bench follow-up. |
| Cancelled running or paused session is recorded once | Core outcome tests and adapter tests; exact physical append is a bench follow-up.                      |
| Pause and resume do not create records               | `intermediate_and_non_session_inputs_emit_no_outcome`.                                                 |
| Sequence survives reboot                             | Journal reconstruction, persistence, and append/reboot tests.                                          |
| Separate devices use independent sequences           | Stable identity/epoch model and SQLite composite-key repository tests.                                 |
| Irrecoverable journal reset                          | Ambiguous-epoch and lost-identity recovery tests.                                                      |
| Preset later changes                                 | Immutable preset snapshot core and journal codec tests.                                                |
| Clock is unknown                                     | Cold-boot clock and nullable timestamp tests across Rust, mock, and SQLite.                            |
| Normal catch-up page                                 | Multi-page foreground-sync and SQLite commit tests.                                                    |
| Client is older than retained history                | Stale-cursor gap model/mock tests and incomplete-history UI model.                                     |
| Client is current                                    | Empty/current foreground-sync test.                                                                    |
| Duplicate page arrives                               | Duplicate-page idempotence tests in mock sync and SQLite.                                              |
| Synchronization stops mid-page                       | Disconnect/retry from last committed cursor and SQL rollback tests.                                    |
| Append persistence fails                             | Bounded retry adapter test; physical timer-usability confirmation is a bench follow-up.                |
| Journal is unreadable at boot                        | Corrupt-slot/metadata recovery tests; injected NVS diagnostic is a bench follow-up.                    |

## BLE device connection

| Scenario                          | Evidence or explicit follow-up                                                                                   |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Foreground discovery              | Native adapter service-filter scan test; physical advertising/discovery is a bench follow-up.                    |
| Radio initialization failure      | Firmware failure path builds; physical timer-authority observation is a bench follow-up.                         |
| Successful connection             | Native adapter discovery/characteristic validation test; physical GATT handshake is a bench follow-up.           |
| Handshake timeout                 | Typed timeout/error path tests; physical timeout is a bench follow-up.                                           |
| Protocol incompatibility          | Protocol/client/mock tests and explicit incompatible UI state.                                                   |
| Disconnect during running session | Link-loss/request rejection tests; simultaneous physical timer observation is a bench follow-up.                 |
| Reconnection after absence        | Remembered transport hint plus SQLite cursor retry tests; physical catch-up is a bench follow-up.                |
| Bluetooth is disabled             | Native adapter availability test and unavailable UI state; physical iOS toggle is a bench follow-up.             |
| Permission is denied              | Native adapter permission test and Settings action; physical denial is a bench follow-up.                        |
| App resumes in range              | Explicit remembered-device reconnect action and full foreground sync path; physical resume is a bench follow-up. |
| No background execution granted   | Native config has no background BLE mode; journal catch-up remains the correctness path.                         |

## Mobile companion

| Scenario                               | Evidence or explicit follow-up                                                                                            |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Develop without hardware               | Deterministic mock scenarios and mobile checks.                                                                           |
| Switch to BLE backend                  | One runtime composition boundary, config tests, signed BLE development build.                                             |
| First successful setup                 | Stable handshake identity is persisted separately from iOS peripheral hint; physical setup is a bench follow-up.          |
| Known device is unavailable            | SQLite is loaded before BLE and explicit reconnect is offered.                                                            |
| Completed history is synchronized      | Mock multi-page sync, actual SQLite repository tests, and history models.                                                 |
| Untimestamped record is shown honestly | Nullable timestamp tests and explicit untimestamped presentation.                                                         |
| Retention gap is visible               | Gap mock, persisted completeness, and incomplete-history presentation.                                                    |
| Disconnect during synchronization      | Link-loss transport test and retry-from-committed-page sync test.                                                         |
| Local database operation fails         | Actual SQLite rollback/retry test and failed sync UI state.                                                               |
| App is absent                          | Durable device journal host/persistence tests; physical reconnect is a bench follow-up.                                   |
| Running status is observed             | Read-only status projection and screen model; physical live status is a bench follow-up.                                  |
| Phone has no internet                  | No network/cloud dependency in transport, repository, or app flow; physical airplane/LAN variation is optional follow-up. |
| Application data is removed            | SQLite owns the only phone copy and no cloud recovery is claimed; iOS uninstall behavior is platform-defined.             |

## Native build evidence

- `react-native-ble-plx` 3.5.1 and `MultiplatformBleAdapter` 0.2.0 were
  autolinked by CocoaPods.
- The generated app contains Bluetooth usage text and no background BLE mode.
- Xcode built and signed scheme `Muninn` for generic iOS with bundle identifier
  `com.jarto666.focustimer`.
- The app was installed on the paired iPhone over CoreDevice `localNetwork`.
- A remote launch while the user was away was rejected only because the iPhone
  was locked; installation remained successful.

## Open physical checklist

These items intentionally remain unchecked in `tasks.md` until the controller
is connected and evidence is recorded:

1. Flash the integrated firmware and repeat the standalone timer smoke test.
2. Prove physical advertising, discovery, GATT subscribe/write/notify, handshake,
   disconnect, and advertising restart.
3. Generate completed/cancelled journal entries, reboot, fill/evict, corrupt, and
   verify exact-once/recovery behavior through the iPhone.
4. Run end-to-end offline catch-up and keep SQLite history after disconnect.
5. Stress sync while rotating/pressing the encoder and observing OLED/buzzer.
6. Measure firmware size, heap, negotiated payload/throughput, resets/watchdog,
   journal capacity, and whole-device USB current.

## Final command ledger

On 2026-08-14, the final tree was copied to an isolated temporary directory
without `.git`, `node_modules`, generated iOS files, or Cargo targets. A fresh
`pnpm install --frozen-lockfile` installed 863 packages, after which
`./scripts/check-all.sh` passed with formatting/lint warnings denied:

- Rust: all `focus-core`, `focus-sync`, `focus-firmware`, and `focus-protocol`
  host tests, Clippy gates, and doc tests passed.
- TypeScript protocol: 5 files / 22 tests passed.
- Mobile: 5 files / 32 tests passed, including native BLE transport and real
  SQLite transaction tests.
- Device client: 4 files / 13 tests passed.
- Deterministic mock: 4 files / 22 tests passed.

The integrated ESP32-C3 production target also built offline with the pinned
ESP-IDF/NimBLE toolchain. Final validation uses:

```sh
pnpm install --frozen-lockfile
./scripts/check-all.sh
./scripts/build-firmware.sh
openspec validate add-ble-device-sync --strict
git diff --check
```

The frozen install, combined check, and firmware build are complete. OpenSpec
strict validation and `git diff --check` are rerun after the final task-file
updates. Physical follow-ups above are not inferred from these host checks.
