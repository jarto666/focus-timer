# BLE sync acceptance ledger

This ledger audits every scenario introduced by OpenSpec change
`add-ble-device-sync`. `Automated` means the behavior is covered by a committed
host/mobile test or strict build. `Native` means the generated iOS graph was
signed and installed on the physical iPhone. `Bench follow-up` is deliberately
not a pass: it names the remaining physical evidence required before archive.

## Product workspace

| Scenario                                                      | Evidence or explicit follow-up                                                                                         |
| ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Developer locates product areas                               | Automated: root layout and links in `README.md`; root scoped check scripts.                                            |
| New application can be added without moving device code again | Automated: independent `device/`, `apps/`, `packages/`, and `protocol/` workspaces.                                    |
| Fresh checkout validation                                     | Automated: frozen pnpm install and root check ledger below.                                                            |
| Firmware flashing remains reproducible                        | Physical: the runner repeatedly inspected and flashed `/dev/cu.usbmodem101`; each run preserves its checksum and logs. |
| Host checkpoint after migration                               | Automated: `check-device`, `check-protocol`, and `check-mobile` gates.                                                 |
| On-device smoke test after migration                          | Existing pre-BLE evidence in `docs/acceptance.md`; BLE-integrated repeat is a bench follow-up.                         |
| Device-only developer                                         | Automated: `scripts/check-device.sh` does not invoke Node.js.                                                          |
| Mobile-only development                                       | Automated: mock backend and TypeScript gates need no ESP hardware.                                                     |

## Device sync protocol

| Scenario                              | Evidence or explicit follow-up                                                                                  |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Compatible handshake                  | Rust/TypeScript fixture and codec tests plus `device-client.test.ts`.                                           |
| Incompatible major version            | Rust protocol-session and TypeScript client/mock tests.                                                         |
| Unknown request type                  | Rust/TypeScript malformed tests and firmware protocol-session recovery test.                                    |
| Golden request fixture                | Cross-language fixture gates in both protocol packages.                                                         |
| Historical fixture remains readable   | Cross-language historical fixture decoding tests.                                                               |
| Oversized message                     | Rust and TypeScript codec/transport bound tests.                                                                |
| Invalid payload                       | Truncation, duplicate-key, non-canonical, checksum, and recovery tests in both languages.                       |
| Fragmented BLE transfer               | Rust/TypeScript multi-frame tests and native-adapter bidirectional fragmentation test.                          |
| Retransmitted read request            | Repeatable journal page and duplicate-page idempotence tests.                                                   |
| Current status read                   | Immutable status projection and typed client API tests.                                                         |
| Clock anchor update                   | Host clock tests, protocol codec tests, foreground sync flow, and physical backwards-anchor rejection/recovery. |
| Remote-control request is unavailable | Command-surface audit: only hello, status, page, and volatile clock-anchor requests exist.                      |

## Session journal sync

| Scenario                                             | Evidence or explicit follow-up                                                                                           |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Completed session is recorded once                   | Physical: offline eight-second Focus completion appended exactly sequence 199 in `20260814T133627Z-acceptance`.           |
| Cancelled running or paused session is recorded once | Physical: paused cancellation appended sequence 196 and degraded-recovery running cancellation appended sequence 197.   |
| Pause and resume do not create records               | `intermediate_and_non_session_inputs_emit_no_outcome`.                                                                   |
| Sequence survives reboot                             | Automated tests plus physical `20260814T130748Z-journal-fill` and the following healthy integrated recovery.             |
| Separate devices use independent sequences           | Stable identity/epoch model and SQLite composite-key repository tests.                                                   |
| Irrecoverable journal reset                          | Ambiguous-epoch and lost-identity recovery tests.                                                                        |
| Preset later changes                                 | Immutable preset snapshot core and journal codec tests.                                                                  |
| Clock is unknown                                     | Physical iPhone History showed real time for the fresh anchored record and the explicit unavailable-time copy for older diagnostic records. |
| Normal catch-up page                                 | Multi-page foreground-sync and SQLite commit tests.                                                                      |
| Client is older than retained history                | Automated UI/model tests plus physical device-side `stale_cursor_gap=true` after 65-record fill; phone catch-up remains. |
| Client is current                                    | Empty/current foreground-sync test.                                                                                      |
| Duplicate page arrives                               | Duplicate-page idempotence tests in mock sync and SQLite.                                                                |
| Synchronization stops mid-page                       | Disconnect/retry from last committed cursor and SQL rollback tests.                                                      |
| Append persistence fails                             | Bounded retry adapter tests plus physical corrupt-journal recovery followed by normal input, cancellation, and completion. |
| Journal is unreadable at boot                        | Physical metadata/slot injection and degraded integrated recovery in `20260814T133525Z` / `133627Z`.                     |

## BLE device connection

| Scenario                          | Evidence or explicit follow-up                                                                                  |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Foreground discovery              | Physical: `20260814T123630Z-ble-echo` was discovered twice by Muninn after advertising start/restart.           |
| Radio initialization failure      | Physical: `20260814T183110Z-radio-failure` ran start/pause/resume/cancel/complete with no BLE host and preserved sequences `202..203`. |
| Successful connection             | Physical echo transport plus production Hello, clock-anchor, and status exchanges at negotiated MTU 185.        |
| Handshake timeout                 | Typed timeout/error path tests; physical timeout is a bench follow-up.                                          |
| Protocol incompatibility          | Protocol/client/mock tests and explicit incompatible UI state.                                                  |
| Disconnect during running session | Physical fault runs dropped a stale multi-fragment response, restarted advertising, and the standalone timer subsequently completed without reset. |
| Reconnection after absence        | Physical remembered-device reconnect and one-record suffix catch-up in `20260814T132027Z-acceptance`.           |
| Bluetooth is disabled             | Physical iPhone: disabling Bluetooth in Settings now transitions live to `Bluetooth is off`, preserves local history, and no longer mislabels the remembered timer as unpaired. |
| Permission is denied              | Native adapter permission test and Settings action; physical denial is a bench follow-up.                       |
| App resumes in range              | Physical locked-to-active remembered-device reconnect and eight-page catch-up in `20260814T132027Z-acceptance`. |
| No background execution granted   | Native config has no background BLE mode; journal catch-up remains the correctness path.                        |

## Mobile companion

| Scenario                               | Evidence or explicit follow-up                                                                                            |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Develop without hardware               | Deterministic mock scenarios and mobile checks.                                                                           |
| Switch to BLE backend                  | One runtime composition boundary, config tests, signed BLE development build.                                             |
| First successful setup                 | Physical production handshake persisted stable identity separately from the iOS peripheral hint.                          |
| Known device is unavailable            | SQLite is loaded before BLE and explicit reconnect is offered.                                                            |
| Completed history is synchronized      | Mock multi-page sync, actual SQLite repository tests, and history models.                                                 |
| Untimestamped record is shown honestly | Nullable timestamp tests and explicit untimestamped presentation.                                                         |
| Retention gap is visible               | Gap mock, persisted completeness, and incomplete-history presentation.                                                    |
| Disconnect during synchronization      | Link-loss transport test and retry-from-committed-page sync test.                                                         |
| Local database operation fails         | Actual SQLite rollback/retry test and failed sync UI state.                                                               |
| App is absent                          | Physical: sequence 199 completed with BLE disconnected, then synchronized into SQLite and remained visible after power-off. |
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
- CoreDevice launches the app over `localNetwork`; while the iPhone is locked,
  iOS immediately suspends foreground BLE/Metro work. The one-shot remembered
  reconnect runs when the app is active and retains an explicit retry fallback.

## Physical BLE echo evidence

Run `docs/hardware-evidence/ble-sync/20260814T123630Z-ble-echo` flashed the
bounded echo image to the ESP32-C3 over `/dev/cu.usbmodem101`. The physical
iPhone running Muninn completed two independent cycles of foreground discovery,
connection, ATT MTU 256 negotiation, response subscription, 23-byte command
write, 23-byte notification, clean disconnect, and advertising restart. The
second cycle proves rediscovery after restart. `monitor.log`, `board-info.log`,
`flash.log`, the exact artifact checksum, and the reviewed observation are kept
with the run. The expected `unsupportedMessage: 1` UI result proves the echo
image did not masquerade as the production Hello responder.

## Firmware size evidence

Both revisions were built from their own clean source tree with the same pinned
ESP toolchain, default production features, and debug profile. Commit `ba49b6d`
is the last journal-integrated checkpoint before the BLE firmware adapter;
`c39dfc7` is the native-iPhone-BLE checkpoint. GNU `size` reports allocatable
ELF sections, not the 21 MB debug-symbol file size:

| Revision                 | `text`   | `data`  | `bss`    | Total (`dec`) |
| ------------------------ | -------- | ------- | -------- | ------------- |
| pre-BLE `ba49b6d`        | 463,896  | 220,558 | 513,282  | 1,197,736     |
| BLE checkpoint `c39dfc7` | 793,468  | 290,593 | 803,685  | 1,887,746     |
| delta                    | +329,572 | +70,035 | +290,403 | +690,010      |
| relative delta           | +71.0%   | +31.8%  | +56.6%   | +57.6%        |

This closes the reproducible pre/post size, heap, stack, watchdog, negotiated
payload, and throughput slices of task 9.4. Flash image
partition fit is now 1,095,184 / 4,128,768 bytes (26.53%). Physical idle after
OLED, 64-record NVS recovery, and BLE initialization reports 137,712 B free
8-bit heap and 30,016 B main-stack reserve with zero unexpected resets. The
physical iPhone run negotiated MTU 185, retained 135,008 B minimum free heap
during eight three-frame pages, completed each 356-byte page in 10 ms, and did
not reduce the main-stack reserve. Whole-device current still requires the
ordered meter; no host ELF number substitutes for it.

## Open physical checklist

These items intentionally remain unchecked in `tasks.md` until their remaining
human-visible evidence is recorded:

1. Stress sync while rotating/pressing the encoder and observing OLED/buzzer;
   the sequential input/cancel path is already recorded in the resource run.
2. Exercise physical iOS Bluetooth-disabled and permission-denied UI states.
3. Measure whole-device USB current when the ordered meter arrives.

Prepare and capture each controller run with
`scripts/ble-acceptance.sh`. The runner preserves the exact ELF and machine
transcripts under `docs/hardware-evidence/ble-sync/`, refuses ambiguous USB
targets, and leaves every OpenSpec checkbox untouched until the visible and
serial evidence has been reviewed. Start with `--mode ble-echo`, use
`--mode acceptance` for short end-to-end sessions, use the three journal modes
only in the documented recovery order, and restore `--mode production` last.

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
