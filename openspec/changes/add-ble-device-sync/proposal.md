## Why

The validated offline timer can already run sessions independently, but it cannot export what happened or support a companion experience while the next hardware revision is being assembled. This is the right point to establish a product-level monorepo and prove one end-to-end iPhone-to-device synchronization path without making connectivity part of timer correctness.

## What Changes

- Reorganize the repository into a product monorepo with the existing Rust workspace under `device/`, mobile code under `apps/mobile/`, and reusable TypeScript packages under `packages/`, while preserving the currently validated firmware behavior and commands through documented root-level entrypoints.
- Introduce one versioned, transport-independent binary protocol with compatible Rust and TypeScript representations, deterministic golden fixtures, explicit size limits, request correlation, and BLE fragmentation/reassembly.
- Add an ESP32-C3 BLE peripheral that advertises a Focus Timer service, performs a versioned handshake, exposes read-only device/session synchronization commands plus an optional clock-anchor command, reports connection state diagnostically, and cannot become the owner of timer state.
- Persist a bounded, epoch-qualified and monotonically sequenced journal of completed and cancelled session outcomes on the device so a phone can catch up after being absent or disconnected.
- Add an Expo/React Native/TypeScript iPhone application built as an Expo Development Build, with mock and BLE transports behind one device-client interface.
- Provide the first useful mobile vertical slice: discover and connect to one timer, synchronize journal records idempotently into local storage, and show connection state plus a simple local session history.
- Keep the physical timer fully usable with no phone, Bluetooth connection, account, server, network, or valid wall-clock source.
- Treat the existing `build-focus-device-mvp` change as an independent 39/40 hardware checkpoint until whole-device USB current is measured; this change does not weaken or close that acceptance requirement.
- Explicitly defer remote start/pause/cancel, runtime preset editing, Wi-Fi, cloud accounts, cross-device cloud sync, OTA, analytics, widgets, Live Activities, production ownership security, battery optimization, the new display adapter, enclosure work, and replacement LED integration.

Assumptions requiring implementation-time validation:

- The current ESP32-C3-MINI-1 carrier has enough flash, RAM, task capacity, and radio stability for ESP-IDF BLE alongside the existing event loop, OLED, encoder, buzzer, and NVS adapters; this must be measured on the connected board before the integrated slice is accepted.
- The first bench protocol may operate without production-grade ownership provisioning because it exposes no Wi-Fi credentials and supports no remote timer-control command. Its discoverability, privacy, bonding, and physical-confirmation model must be revisited before external distribution.
- iOS background BLE behavior is opportunistic. The device journal, not background execution, is the source of durability, and acceptance is based on reliable catch-up when the app is foregrounded or reconnected.
- Session records may lack wall-clock timestamps until a phone supplies a clock anchor. Sequence, preset snapshot, duration, and outcome remain valid without a timestamp.

## Capabilities

### New Capabilities

- `product-workspace`: Preserve reproducible device and mobile workflows while reorganizing the repository into an extensible product monorepo.
- `device-sync-protocol`: Define compatible, versioned Rust/TypeScript messages, size limits, framing, compatibility behavior, and golden fixtures independent of BLE or future Wi-Fi transport.
- `ble-device-connection`: Discover, connect, handshake, observe, disconnect, and recover a single Focus Timer over BLE without affecting standalone timing.
- `session-journal-sync`: Persist bounded session outcomes on the device and synchronize them idempotently into the phone's local store by device identity, journal epoch, and monotonic sequence.
- `mobile-companion`: Provide an iPhone-first Expo application that works against mock and real devices and presents connection state plus locally synchronized session history.

### Modified Capabilities

None. The existing offline focus-session contract remains unchanged; lifecycle outcomes are consumed by the new journal capability after the core commits its existing transitions.

## Impact

- Moves the existing Cargo workspace and Rust crates under `device/`; repository documentation, CI/developer commands, paths, build artifacts, ESP-IDF configuration, and flash instructions must be updated atomically and verified before BLE work begins.
- Adds a Rust protocol crate, firmware BLE and journal adapters, generated protocol artifacts or code-generation tooling, NVS schema additions, and host/on-device integration tests.
- Adds Node.js/pnpm workspace metadata, an Expo/React Native application, reusable TypeScript packages, SQLite-backed local persistence, BLE native dependencies, Xcode project generation, and iPhone development-build instructions.
- Extends `focus-core` effects or an equivalent domain boundary to report completed/cancelled outcomes without introducing radio, serialization, storage, or wall-clock dependencies into the core crate.
- Creates a wire-compatibility obligation: released protocol versions and golden fixtures must not be changed incompatibly without a deliberate version transition.
- Adds no cloud infrastructure, public API, account system, desktop application, mandatory network dependency, or new requirement for a phone to run the timer.
