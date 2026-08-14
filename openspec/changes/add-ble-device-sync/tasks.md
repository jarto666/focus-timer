## 1. Preserve the Device Checkpoint and Create the Monorepo

- [x] 1.1 Preserve the current documentation changes and the open `build-focus-device-mvp` 39/40 state, run the existing host suite, and record a pre-move firmware/toolchain checkpoint without marking its unmeasured power-budget task complete.
- [x] 1.2 Move the root Cargo workspace, lockfile, `focus-core`, `focus-firmware`, ESP-IDF configuration, and target-specific files under `device/` as one mechanical path-only change.
- [x] 1.3 Add the canonical `apps/`, `packages/`, `protocol/`, and `scripts/` roots plus root documentation explaining the product layout and the independent Rust and TypeScript prerequisites.
- [x] 1.4 Add root-invocable device, mobile, protocol, and combined check scripts that preserve device-only operation without Node.js and mock-mobile operation without ESP hardware.
- [x] 1.5 Update every repository reference to Cargo, build artifacts, diagnostics, flash/monitor commands, evidence images, and crate paths; verify no stale canonical root-Cargo path remains.
- [x] 1.6 Run formatting, lint, all existing host tests, default firmware build, and every retained diagnostic feature build from the reorganized paths and record the passing path-migration checkpoint.
- [x] 1.7 Flash the default path-migrated firmware and repeat the offline select/start/pause/resume/cancel/complete/dismiss/reboot smoke test before adding protocol or radio dependencies.

## 2. Establish the TypeScript and Mock-Mobile Foundation

- [x] 2.1 Add a pinned pnpm workspace for `apps/mobile`, `packages/device-protocol`, `packages/device-client`, and `packages/mock-device`, with shared strict TypeScript, formatting, lint, and test configuration.
- [x] 2.2 Scaffold the Expo/React Native/TypeScript application with Expo Router, document mock versus physical configuration, and keep screen code independent of the selected transport.
- [x] 2.3 Produce and install a minimal Expo Development Build on the physical iPhone through Xcode, then pin the proven Node.js, pnpm, Expo SDK, React Native, iOS target, and native dependency versions.
- [x] 2.4 Define and test the transport-neutral connection-state and `DeviceTransport` contracts, including unavailable, permission-denied, scanning, connecting, handshaking, ready, disconnected, incompatible, and retryable-error states.
- [x] 2.5 Implement a deterministic mock transport/device with empty, paged, duplicated, gap, changed-epoch, disconnect, incompatible-version, malformed-response, and unavailable-device scenarios.
- [x] 2.6 Build the first mock-backed device setup/status and local-history screen states, including honest untimestamped and incomplete-history presentation, without adding remote timer controls.

## 3. Define and Prove the Cross-Language Protocol

- [x] 3.1 Write the protocol schema registry with service/characteristic UUIDs, protocol version policy, numeric message/field identifiers, constrained-CBOR rules, string/page/message limits, error codes, and BLE frame format.
- [x] 3.2 Add the `focus-protocol` Rust crate with bounded logical models and explicit canonical codecs for handshake, status, session pages, clock anchors, and structured errors.
- [x] 3.3 Implement the matching TypeScript logical models and explicit canonical codecs in `packages/device-protocol` without generic object serialization.
- [x] 3.4 Implement bounded BLE fragmentation and reassembly codecs in Rust and TypeScript with transfer correlation, contiguous offsets, checksum validation, timeout reset, and one in-progress transfer limit.
- [x] 3.5 Commit shared golden fixtures for every message kind and edge value, and gate both Rust and TypeScript encoders/decoders on byte-identical output and historical-fixture decoding.
- [x] 3.6 Add malformed, truncated, duplicate-key, out-of-order-fragment, checksum, unsupported-message, incompatible-version, and oversized-input tests proving bounded recovery with no panic.
- [x] 3.7 Add the protocol compatibility check to the root workflows and document the rule that committed identifiers and fixtures require an explicit version transition rather than in-place reinterpretation.

## 4. Emit Session Outcomes and Model the Journal on the Host

- [x] 4.1 Extend the pure core effect boundary with Completed and Cancelled lifecycle outcomes containing the immutable preset snapshot, planned duration, and accumulated active duration while retaining `no_std` and all existing behavior.
- [x] 4.2 Add core tests proving exactly one completion outcome, one running/paused cancellation outcome, paused-time exclusion, and no outcome for pause, resume, dismissal, selection, refresh, or ignored input.
- [x] 4.3 Implement a host-testable fixed-capacity journal model with stable device identity, journal epoch, monotonic sequence, eviction, ascending bounded pages, continuation, changed-epoch behavior, and explicit stale-cursor gaps.
- [x] 4.4 Implement a host-testable volatile UTC-to-monotonic clock anchor and optional session timestamp derivation, including cold-boot unknown time and backwards/overflow rejection.
- [x] 4.5 Define the immutable device-status projection consumed by the protocol, including timer view state, preset snapshot, remaining duration, journal bounds/health, and clock-known state without exposing mutable `App` access.
- [x] 4.6 Add host tests covering journal wrap, reboot reconstruction inputs, duplicate page reads, current cursors, stale cursors, epoch reset, timestamped/untimestamped records, invalid anchors, and journal-unavailable status.

## 5. Persist and Integrate the Device Journal

- [x] 5.1 Inspect and document the actual NVS partition size and entry overhead, measure the encoded maximum record, select the evidence-backed fixed capacity from the initial target of 64, and keep the existing settings namespace unchanged.
- [x] 5.2 Implement versioned checksummed codecs and redundant metadata for the separate `focus_sync` namespace, including provisioned device identity, journal epoch, high-water position, and fixed slots.
- [x] 5.3 Implement NVS initialization, atomic slot-first append, metadata commit, boot reconciliation, eviction, bounded page reads, stable identity recovery, and new-epoch recovery from irrecoverable journal ambiguity.
- [x] 5.4 Queue committed core outcomes to the journal only after state transition, attach optional clock-derived timestamps, retry at most one failed pending append with bounded backoff, and expose degradation without blocking the event loop.
- [x] 5.5 Add host adapter tests for first boot, append/reboot, power-loss commit boundaries, corrupt slot, corrupt metadata, lost identity, new epoch, failed append, retry, eviction, and selected-preset namespace isolation.
- [x] 5.6 Add focused journal clear/corrupt/fill diagnostics that cannot alter the production timer contract and document safe commands and expected recovery logs.
- [ ] 5.7 Validate on hardware that completion and running/paused cancellation append exactly once, records survive reboot, eviction reports a gap, clock-known and clock-unknown records differ correctly, and injected journal failure leaves the timer usable.

## 6. Bring Up the ESP32-C3 BLE Service

- [x] 6.1 Spike the maintained Rust surfaces over the pinned ESP-IDF NimBLE stack, prove a clean compile, and pin the smallest option that supports the required server callbacks or document the narrow `esp-idf-sys` fallback.
- [ ] 6.2 Advertise the custom Focus Timer service and prove physical iPhone discovery, connect, service discovery, command write, response notification, disconnect, and restart advertising with a bounded echo/handshake diagnostic.
- [x] 6.3 Implement the production BLE adapter, command/response characteristics, handshake gate, structured connection lifecycle, request timeouts, and diagnostic reporting without granting callbacks access to `App` or GPIO.
- [x] 6.4 Connect protocol requests to the immutable status cache, volatile clock anchor, and stateless journal-page reader through bounded mailboxes or copied snapshots.
- [x] 6.5 Integrate Rust fragmentation/reassembly at the GATT boundary and verify multi-fragment responses at default and negotiated payload sizes plus recovery after invalid transfers.
- [ ] 6.6 Verify radio initialization failure, incompatible handshake, unknown command, malformed payload, disconnect, and notification failure cannot affect physical input, monotonic timing, OLED, buzzer, settings, or journal integrity.

## 7. Implement Local Mobile Persistence and Synchronization

- [ ] 7.1 Add the local SQLite schema and migrations for known devices, journal epochs/cursors, gap metadata, and session records keyed by `(device_id, journal_epoch, sequence)` with nullable session timestamps.
- [ ] 7.2 Implement and test repositories that atomically upsert one page, record its gap state, and advance the matching cursor only in the same successful transaction.
- [ ] 7.3 Implement `DeviceClient` handshake, status, clock-anchor, page, compatibility, timeout, and correlated-error APIs on top of `DeviceTransport`.
- [ ] 7.4 Implement the foreground sync state machine that handles empty/current history, multiple pages, duplicate pages, disconnect mid-sync, local commit failure, stale cursor, and changed journal epoch.
- [ ] 7.5 Run the complete persistence and sync suite against the deterministic mock, proving idempotence, retry from the last committed position, offline history, and no fabricated timestamp or cloud recovery.
- [ ] 7.6 Connect device setup/status and history screens to the SQLite-backed application models with explicit empty, syncing, unavailable, incompatible, incomplete, failed, retry, completed, cancelled, and untimestamped states.

## 8. Connect the Physical iPhone over BLE

- [ ] 8.1 Add and configure the pinned BLE native dependency in the Expo Development Build, including iOS Bluetooth usage text, required capabilities for the chosen foreground MVP, and documented rebuild commands; do not depend on Expo Go.
- [ ] 8.2 Implement foreground service-filtered scanning, candidate selection, connect/disconnect, characteristic discovery, handshake, stable-device association, and remembered-device reconnection without treating the iOS peripheral identifier as product identity.
- [ ] 8.3 Implement the mobile BLE request path with pre-subscribed notifications, bounded fragmentation/reassembly, one in-flight request, explicit timeouts, cancellation, and recovery after malformed or partial responses.
- [ ] 8.4 Integrate foreground/resume orchestration: reconnect or offer retry, handshake, send the volatile clock anchor, read status, synchronize to current, and preserve local history when the device is unavailable.
- [ ] 8.5 Prove the same application build can switch between mock and BLE transports without screen branches or divergent logical models.
- [ ] 8.6 Run a physical end-to-end session: operate the timer without the app, reconnect the iPhone, synchronize the new outcome into SQLite, disconnect, and verify the history remains available offline.

## 9. Reliability, Resource, and Handoff Acceptance

- [ ] 9.1 Run and record all root device, mobile, protocol, and combined non-hardware checks from a fresh dependency install with formatting and lint warnings treated as failures.
- [ ] 9.2 Exercise physical duplicate request, multi-page catch-up, disconnect mid-page, reconnect, stale cursor/gap, changed epoch, unknown time, corrupt frame, incompatible version, Bluetooth disabled, and permission-denied scenarios against their capability requirements.
- [ ] 9.3 Run simultaneous large synchronization, fast/slow encoder rotation, OLED refresh, start/pause/resume/cancel/completion, buzzer feedback, reboot, and radio-failure tests and confirm the standalone timer remains authoritative.
- [ ] 9.4 Record pre/post-BLE firmware size, free heap at idle/connected/transferring, watchdog/reset behavior, negotiated BLE payload/throughput, evidence-backed journal capacity, and whole-device USB current when the ordered meter is available.
- [ ] 9.5 Audit the shipped command surface and logs to confirm there is no remote timer control, preset mutation, Wi-Fi credential path, OTA path, cloud/account identifier, advertised stable device identity, or default journal-payload logging.
- [ ] 9.6 Finalize monorepo, protocol, NVS recovery, BLE development-build, mock mode, iPhone setup, test, build, flash, monitor, troubleshooting, security-boundary, and foreground-sync documentation.
- [ ] 9.7 Audit every scenario in `product-workspace`, `device-sync-protocol`, `ble-device-connection`, `session-journal-sync`, and `mobile-companion`, recording evidence or an explicit follow-up before this change is archived.
