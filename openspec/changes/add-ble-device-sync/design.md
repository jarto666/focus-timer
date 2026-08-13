## Context

See `proposal.md` for motivation and scope. The repository currently has one root Cargo workspace containing `focus-core` and `focus-firmware`, a validated single-owner offline event loop, NVS-backed selected-preset settings, hardware diagnostics, and an unfinished hardware acceptance change at 39/40. There is no JavaScript workspace, mobile application, device identity, session journal, BLE service, wall clock, or wire contract.

The current controller is an ESP32-C3-MINI-1 carrier using `std + ESP-IDF`. BLE work must coexist with a one-millisecond encoder sampling path and must not let radio callbacks, serialization, NVS access, or phone lifecycle become owners of `focus_core::App`. The iPhone client must remain useful when hardware is unavailable, and iOS cannot be assumed to provide continuous background execution.

The specifications in `specs/` define the externally observable contract. This design establishes a small first implementation while leaving Wi-Fi and broader application features for later changes.

## Goals / Non-Goals

**Goals:**

- Create stable product directories once, with independent Rust and TypeScript toolchains and no heavy monorepo orchestrator.
- Preserve a reproducible pre-BLE firmware checkpoint through the repository move.
- Share logical protocol meaning and compatibility fixtures between Rust and TypeScript without sharing runtime or transport code.
- Keep one authoritative timer-state owner while publishing read-only status and committed lifecycle outcomes to optional adapters.
- Make device history durable enough for foreground catch-up and safe under repeated, interrupted, and stale synchronization.
- Prove the path on a physical iPhone and the current ESP32-C3, while keeping most mobile work runnable against a deterministic mock.

**Non-Goals:**

- Abstracting every future application into a universal UI or forcing React Native components into a desktop shell now.
- Supporting multiple simultaneous centrals, multiple configured devices in the first UI, remote timer control, or live phone-owned countdown truth.
- Guaranteeing background discovery, cloud recovery, production ownership security, encrypted application payloads, or cross-phone synchronization.
- Migrating the selected-preset NVS record or changing existing GPIO, display, encoder, buzzer, diagnostic, and session semantics during the directory move.
- Coupling the new display, perfboard, battery, enclosure, or replacement LED work to BLE/mobile implementation.

## Decisions

### 1. Use a simple product monorepo with `device/`, `apps/`, `packages/`, and `protocol/`

The target layout is:

```text
focus-timer/
  apps/
    mobile/                  Expo application
  device/
    Cargo.toml
    Cargo.lock
    crates/
      focus-core/
      focus-firmware/
      focus-protocol/
  packages/
    device-protocol/         TypeScript codecs and logical messages
    device-client/           transport-neutral client and sync state machine
    mock-device/             deterministic test backend
  protocol/
    schema.md                numeric field registry and compatibility rules
    fixtures/                canonical cross-language byte fixtures
  scripts/                   root-invocable scoped checks
  docs/
  openspec/
  package.json
  pnpm-workspace.yaml
```

The Rust workspace is genuinely rooted under `device/`; it is not kept at repository root with long member paths. Small POSIX scripts provide root-invocable device, mobile, protocol, and combined checks so a device-only workflow does not require Node.js. The TypeScript workspace uses pnpm and exact lockfile versions. Nx, Turborepo, Bazel, and a custom task daemon are unnecessary for three packages and one app.

The move is performed as a behavior-preserving checkpoint before new dependencies are added. Documentation, `.gitignore`, Cargo paths, ESP-IDF build paths, flash commands, diagnostic commands, and any editor/CI configuration change in the same task. Both host checks and one default-firmware physical smoke test gate the migration.

Alternative considered: keep Cargo at root and add `apps/`. That minimizes the move but leaves the repository conceptually firmware-first and does not satisfy the desired durable product boundary. Alternative considered: put everything under `firmware/`. That misnames `focus-core` and the transport-neutral protocol crate.

### 2. Encode logical messages as deterministic constrained CBOR

`focus-protocol` and `@focus-timer/device-protocol` implement the same logical schema using CBOR with these restrictions:

- definite-length maps and arrays only;
- unsigned numeric field and message identifiers registered in `protocol/schema.md`;
- map keys emitted in ascending numeric order;
- integers, booleans, byte strings, bounded UTF-8 strings, arrays, and maps only;
- no floats, indefinite values, duplicate keys, or implementation-specific tagged objects;
- unknown optional fields ignored within a compatible major version;
- unknown required message kinds rejected explicitly.

The envelope contains protocol major/minor, request identifier, message kind, and payload. Initial logical messages are `Hello`, `GetStatus`, `GetSessionPage`, `SetClockAnchor`, successful responses, and structured errors. Status is a read-only snapshot; `SetClockAnchor` updates only the optional UTC-to-monotonic anchor.

Starting limits are a 2 KiB logical message, 32-byte preset identifiers/names, one request awaiting a response per connection, and at most eight records per page. These are named protocol constants and must be reduced rather than silently exceeded if the resource spike disproves them. Every message kind receives Rust round-trip tests, TypeScript round-trip tests, and shared golden bytes.

Alternative considered: JSON. It is easy to inspect but consumes more BLE bandwidth and permits too many ambiguous representations. Alternative considered: Protobuf. It provides good cross-language generation but adds a schema compiler and does not by itself give the byte-canonical fixture contract desired here. Alternative considered: a handwritten positional binary format. It is compact but harder to evolve safely than integer-keyed maps.

### 3. Keep BLE fragmentation outside the logical protocol

The device exposes one custom 128-bit Focus Timer service with:

- a command characteristic accepting writes with response;
- a response characteristic supporting notifications;
- no mutable timer-control characteristic.

Advertising contains the service UUID and a short product name, not the stable product identifier. iOS receives the stable identity only from `Hello`; the platform-specific peripheral identifier is only a reconnection hint.

Each GATT value carries a small frame header containing frame version, flags, transfer identifier, total logical length, fragment offset, and whole-message checksum. Payload size is derived from the effective characteristic payload, so the design works at the default ATT size and after MTU negotiation. Receivers accept one bounded in-progress transfer per connection, require contiguous offsets, and reset safely on timeout, duplicate start, order violation, checksum failure, or overflow.

The mobile transport writes one complete request transfer, subscribes before requesting, reassembles one correlated response, and applies explicit connection/request timeouts. A retry repeats the logical read request; the device holds no destructive cursor state.

Alternative considered: one characteristic with request/response polling. Notifications give a cleaner response path and avoid depending on large characteristic reads. Alternative considered: expose each field as a separate characteristic. That bakes the application schema into GATT and makes future Wi-Fi reuse harder.

### 4. Use ESP-IDF NimBLE behind a narrow firmware adapter and mailbox boundary

The firmware continues to have one task that owns `App`. After every committed transition it publishes the newest immutable status snapshot to a small read-only cache. BLE callbacks can read that cache and query the journal but cannot call `App::handle`, access GPIO, or execute session transitions.

`SetClockAnchor` updates a separate clock-anchor service through a bounded mailbox. Journal-page reads operate on immutable copied records or a storage-owned request queue. Radio and protocol errors are logged and surfaced in status diagnostics; they never stop the main event loop.

The implementation uses the ESP-IDF NimBLE stack. A first compile/advertise spike selects and pins the smallest maintained Rust surface that works with the repository's ESP-IDF version; if a high-level crate cannot satisfy the framing and lifecycle requirements, the adapter may use `esp-idf-sys` bindings without changing the rest of the design. That crate choice is isolated and does not affect capability behavior or task ordering.

Resource acceptance records pre/post-BLE firmware size, free heap at idle and while connected/synchronizing, encoder event reliability during transfers, reset/watchdog behavior, and USB current once the ordered meter is available.

Alternative considered: make the BLE task a second application-state owner protected by a mutex. That creates ordering ambiguity with physical input and violates the existing architecture. Alternative considered: add Wi-Fi at the same time. It expands provisioning, credentials, power, discovery, and security before the BLE vertical slice is proven.

### 5. Emit committed lifecycle outcomes from the core boundary without adding I/O

`focus-core` remains `no_std` and unaware of BLE, CBOR, UTC, NVS, device identity, and mobile records. Its effects gain an optional semantic outcome emitted only after these existing transitions commit:

- first `Running -> Completed` transition;
- `Running -> Idle` cancellation;
- `Paused -> Idle` cancellation.

The outcome contains the immutable preset snapshot, outcome kind, planned duration, and accumulated active duration. Paused time is excluded because remaining duration is frozen while paused. Refreshes, completion dismissal, pause, resume, selection, and ignored inputs emit no outcome. Existing transition tests are extended to prove exactly-once behavior.

The firmware adapter combines that outcome with device-owned sequence/epoch and any available UTC anchor, then queues a journal append. The session state is already committed before the append is attempted, so storage failure cannot roll it back.

Alternative considered: infer outcomes by comparing display snapshots in the BLE or journal task. That duplicates state-machine knowledge and risks recording dismissal or reconnect behavior. Alternative considered: put journal writes in `focus-core`. That would violate its pure host-testable boundary.

### 6. Store a recoverable fixed-capacity journal in a separate NVS namespace

The `focus_sync` namespace is independent of the existing selected-preset settings. It contains:

- a randomly provisioned 128-bit stable device identifier;
- a randomly provisioned 64-bit journal epoch;
- redundant versioned metadata/high-water records;
- a fixed number of versioned, checksummed record slots.

The starting capacity is 64 records. Each append writes and commits the target slot before advancing redundant metadata. On boot, valid slots and metadata are reconciled; the highest valid sequence is authoritative. Eviction overwrites the oldest slot only after the next sequence is assigned. An irrecoverable reset creates a new random journal epoch, so a sequence restart cannot collide with earlier phone data. Loss of the separate device-identity record makes the board a new logical device and is reported explicitly.

Record strings and durations are copied from the active preset snapshot. The firmware keeps at most one pending failed append in RAM and retries with bounded backoff while powered; if durability cannot be established it reports journal degradation and never claims the record through sync.

Page requests are stateless: `(journal_epoch, after_sequence, limit)`. Responses include epoch, oldest/latest retained sequence, gap status, ascending records, and continuation. A changed epoch is handled as a new journal generation rather than silently advancing the previous cursor.

Alternative considered: rewrite one blob containing the entire history. It increases flash churn and makes power-loss recovery coarse. Alternative considered: persist an unbounded log. The prototype's partition is finite and the phone is the long-term store, so a documented ring is the correct ownership boundary.

### 7. Treat wall-clock time as an optional volatile anchor

The timer remains monotonic. A valid `SetClockAnchor` supplies `(utc_ms, device_monotonic_ms_at_receipt)` and is held only for the current boot. When a session starts and ends while an anchor is known, the firmware derives optional UTC timestamps from monotonic deltas. A cold boot returns to `clock_unknown` until another anchor arrives; it does not trust persisted wall time across power loss.

The mobile database stores timestamps as nullable. Untimestamped records remain sortable by journal position and are displayed without a fabricated calendar date. The app may record `synced_at` as metadata but cannot substitute it for the session time.

Alternative considered: persist the phone time and advance it after reboot. Without a battery-backed RTC or a trustworthy elapsed-time source across full power loss, that creates confidently wrong dates. Alternative considered: require time before starting a session. That would make the phone part of the critical timer path.

### 8. Build the iPhone client with Expo Development Builds and transport-neutral packages

The JavaScript workspace uses TypeScript, Expo, React Native, and Expo Router. BLE requires a native development build, not Expo Go. The physical transport uses `react-native-ble-plx`; local persistence uses `expo-sqlite`. Native dependencies and Expo SDK versions are pinned together after a minimal Xcode/iPhone build succeeds.

The package responsibilities are:

```text
device-protocol   bytes <-> logical messages and fixtures
device-client     connection state, request API, page sync algorithm
mock-device       deterministic discovery/transport/journal scenarios
apps/mobile       BLE adapter, SQLite repository, screens and platform lifecycle
```

`DeviceTransport` handles connection plus request bytes. `DeviceClient` performs handshake and exposes typed status/page calls. The sync engine imports one page in a SQLite transaction and advances the cursor only in that same transaction. Tables use `(device_id, journal_epoch, sequence)` as the record key and retain separate per-epoch gap metadata. Screens consume application models rather than BLE objects.

The first screens are device setup/status and local history. They have explicit empty, unavailable, incompatible, syncing, incomplete-history, and retry states. Mock mode is selected by documented development configuration, not by screen-specific branches.

Alternative considered: native SwiftUI. It offers direct CoreBluetooth integration but abandons the user's TypeScript/React stack and immediate Android path. Alternative considered: Capacitor. It maximizes DOM reuse but makes a WebView the primary mobile UI and offers a thinner BLE ecosystem for this device-first app. Alternative considered: Electron. It does not target iOS.

### 9. Make foreground catch-up the correctness path

On foreground launch or explicit retry, the app restores the known device, establishes BLE, handshakes, sends a clock anchor, reads status, and requests pages after the last committed position until current. On disconnect it preserves committed data and stops; reconnect resumes from SQLite. It does not require continuous scans, a background timer, or a JavaScript process kept alive by unsupported techniques.

iOS background modes and state restoration can be explored after foreground reliability is proven, but they are performance enhancements. The durable device journal is the only contract that closes the absence gap.

Alternative considered: require background completion pushes. That makes correctness depend on OS scheduling and creates a false guarantee. Alternative considered: keep a live phone-side countdown. That would create competing time truth and unnecessary background pressure.

### 10. Limit the bench security surface and require a later ownership change

This change advertises a development BLE service without product ownership provisioning. It contains no Wi-Fi credentials, cloud tokens, account identifiers, remote controls, preset mutation, or OTA path. The clock-anchor command accepts only bounded UTC data and cannot affect monotonic session accounting. Logs avoid dumping journal payloads by default.

This is acceptable only for a personal bench prototype. Before external distribution or adding sensitive/mutating commands, a separate change must define physical confirmation, ownership reset, bonding/encryption, privacy-preserving advertising, and authorization per command.

Alternative considered: design full production pairing now. That would expand UI, recovery, reset, multi-owner, and security acceptance before the read-only sync path is proven.

## Risks / Trade-offs

- [The directory move obscures whether later failures come from paths or BLE] → Complete and verify the monorepo migration as its own commit/checkpoint before adding protocol or radio dependencies.
- [Current ESP-IDF Rust BLE wrappers are incomplete or version-incompatible] → Prove compile, advertise, connect, and one echo response first; isolate the chosen NimBLE surface behind the same adapter and pin the working dependency set.
- [BLE work starves one-millisecond encoder sampling or resets the controller] → Keep callbacks bounded, move encoding/storage out of callbacks, measure heap and watchdog behavior, and run encoder/OLED/buzzer interaction during large transfers.
- [NVS capacity or entry overhead is smaller than estimated] → Inspect the actual partition and measure encoded records before fixing the documented capacity; retain the bounded ring/page contract even if the starting capacity is reduced.
- [Power loss occurs between slot and metadata commits] → Use checksummed versioned slots, redundant metadata, boot reconciliation, and epoch rotation on irrecoverable ambiguity.
- [An outcome is committed but cannot be journaled] → Preserve timer correctness, retry one pending append, expose degradation, and never report the missing record as synchronized.
- [Canonical CBOR libraries encode edge cases differently] → Restrict the supported value subset, write explicit codecs rather than generic object serialization, and gate both languages on shared fixtures.
- [iOS identifiers or background behavior are mistaken for device identity/durability] → Derive identity only from `Hello`, persist it locally, and use foreground journal catch-up as the acceptance path.
- [Open development BLE service leaks personal session history to a nearby client] → Keep the command surface read-oriented and personal-bench-only, avoid advertising stable identity, and block distribution until ownership security is specified.
- [The old hardware MVP and new change overlap in the dirty worktree] → Preserve both OpenSpec directories, avoid marking task 8.1 complete, and keep the offline acceptance evidence independently reproducible from the pre-BLE checkpoint.

## Migration Plan

1. Preserve the current dirty documentation deliberately, run the existing host suite, record the current firmware binary/toolchain checkpoint, and ensure the pending hardware task remains open.
2. Move the Cargo workspace, crates, lockfile, and target-specific configuration under `device/` in one mechanical change; update every documented path and add scoped root scripts.
3. Re-run all host checks, every diagnostic build, the default firmware build, and the physical offline smoke test. Roll back only the move if behavior or tooling differs.
4. Add the pnpm workspace and Expo application with a mock-only device client. Pin the successful Xcode/Expo/native dependency set after installing a development build on the iPhone.
5. Add the schema registry, Rust/TypeScript protocol codecs, fragmentation codecs, fuzz/property-style malformed-input tests, and shared fixtures before exposing a radio service.
6. Add core lifecycle outcomes and the host-tested journal model/storage adapter. Use a new NVS namespace so removing the BLE firmware leaves the existing selected-preset record untouched.
7. Add the minimal BLE echo/handshake spike, then status, clock anchor, and journal paging. At each step verify the offline timer without a phone.
8. Connect the real BLE transport to the already mock-tested mobile client and SQLite sync engine; run disconnect, duplicate, gap, epoch reset, corrupt input, and foreground-resume acceptance on the physical iPhone/controller pair.
9. Capture firmware size, heap, radio stability, encoder reliability, and available power measurements; rebuild and flash the default integrated firmware only after the resource gates pass.

Rollback is code-level and data-safe: revert the BLE/runtime integration while retaining the separately verified monorepo move, or flash the preserved pre-BLE firmware. The new `focus_sync` NVS namespace can be ignored or erased without altering the existing selected-preset namespace. The mobile database is new and has no server-side migration obligation.

## Open Questions

- Which maintained Rust binding over ESP-IDF NimBLE compiles cleanly with the pinned toolchain and provides the required callback lifecycle without unnecessary features?
- What journal capacity fits the controller's actual NVS partition after measuring encoded record and entry overhead: the initial target is 64, but the documented value must follow evidence.
- What effective ATT payload and connection parameters does the current iPhone/controller pair negotiate, and how do they affect throughput and heap at the 2 KiB logical-message cap?
- Does enabling BLE materially change the ordered USB power budget or expose a need to defer continuous advertising until the battery/power change?
