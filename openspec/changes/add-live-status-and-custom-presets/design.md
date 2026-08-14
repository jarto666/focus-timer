## Context

See `proposal.md` for motivation and scope. The current product has a host-tested Rust session core, an ESP-IDF firmware loop that exclusively owns `focus_core::App`, a read-oriented CBOR protocol at version 1.0, command/response BLE characteristics, a durable device journal, and a React Native iPhone app that reconnects and performs foreground history catch-up.

`GetStatus` already returns an immutable timer projection, but only as part of a foreground synchronization pass. The BLE response channel permits one correlated request at a time and therefore cannot safely carry unsolicited state alongside responses. The core catalog is currently a borrowed compile-time slice of `&'static str` presets, and selected-preset settings store only a stable identifier. No mutable catalog or physical-confirmation state exists.

The implementation must preserve one-millisecond encoder sampling, monotonic timer ownership, existing selected-settings and journal namespaces, protocol 1.0 clients, and useful standalone behavior through every phone, radio, serialization, and storage failure.

## Goals / Non-Goals

**Goals:**

- Add an ordered, recoverable, full-snapshot observation stream without introducing another timer-state owner.
- Make custom presets owned and persisted by the device after explicit physical approval.
- Keep all new domain behavior fixed-capacity, `no_std`, deterministic, and host-testable.
- Preserve additive compatibility for existing app/firmware pairs and existing NVS data.
- Give the mobile UI explicit draft, confirmation, conflict, stale, and recovery states rather than optimistic ambiguity.

**Non-Goals:**

- Sending session lifecycle commands from the phone or treating local interpolation as countdown truth.
- Supporting concurrent phones, account ownership, cloud merge, background-always-on BLE, or Wi-Fi.
- Designing production pairing/reset/bonding policy beyond physical confirmation of this one mutating operation.
- Making arbitrary schedules, multi-stage routines, sounds, colors, or per-preset automation configurable.
- Changing session-history identity, ordering, retention, or wall-clock semantics.

## Decisions

### 1. Extend protocol 1 with an additive minor and a dedicated event characteristic

Protocol 1.1 adds capabilities for `LiveStatus`, `ReadPresetCatalog`, and `ProposePresetCatalog`, plus catalog request/response messages and unsolicited event messages. Existing 1.0 message identifiers and golden bytes remain unchanged. A device negotiates the lower minor and advertises only capabilities valid for that peer.

The Focus Timer GATT service gains one notify-only event characteristic. Command writes and correlated responses remain on their existing characteristics; events therefore cannot be mistaken for the response to the one in-flight request or delay a catalog/status read. The existing BLE frame format is reused on the event characteristic, with a separate reassembler and transfer-ID sequence. Event envelopes reserve `request_id = 0`; request and response envelopes continue to require a non-zero identifier.

The new event kinds are a full `LiveStatusEvent` and `PresetCatalogResultEvent`. Catalog reads and proposals remain normal correlated requests. Numeric assignments, field bounds, minor-version rules, and new canonical fixtures are committed in `protocol/schema.md` before adapters are changed.

Alternative considered: send events through the response characteristic. It would require multiplexing unsolicited messages with a correlated request and creates avoidable ordering and timeout ambiguity. Alternative considered: poll `GetStatus` every second. It wastes request/response bandwidth, makes interaction latency depend on polling, and complicates catalog operations sharing the sole in-flight request.

### 2. Make every live event a complete ordered snapshot

Firmware creates a random 64-bit `status_epoch` at boot and maintains an unsigned `status_revision`. `GetStatusResponse` gains optional 1.1 fields for both values. Each `LiveStatusEvent` contains the same complete status projection and both ordering fields. A revision increments for every meaningful selection/lifecycle transition and for each one-second Running sample; it does not increment merely because a client connects.

The app subscribes first, reads `GetStatus`, and then accepts the newest epoch/revision from either path. Within one epoch it ignores duplicates and older revisions. If it observes a revision gap, resumes, reconnects, or detects malformed event framing, it reads another full status. It never attempts to replay missing transitions because session history remains the durable record of outcomes.

The mobile presentation may subtract local monotonic elapsed time between Running snapshots solely to animate seconds. Every device snapshot corrects it. Disconnect freezes the authoritative snapshot and marks it stale.

Alternative considered: send deltas such as `Paused` or `Tick`. Deltas are smaller but make correctness depend on observing every notification and require a replay state machine. Alternative considered: use only a revision without an epoch. A reboot could make a valid new low revision look stale.

### 3. Coalesce event delivery behind the existing single-owner firmware loop

Only the main runtime task reads or mutates `App`. After an accepted physical input or periodic Running refresh it copies the newest protocol projection into a fixed-size latest-value slot. A BLE event sender reads that immutable slot and may replace an unsent older snapshot with a newer one. It never queues an unbounded series, invokes `App::handle`, or blocks the main loop waiting for a central.

Connection/subscription state is reported through bounded signals. Encoding and notification happen outside the input sampling path. With no subscribed central, publication becomes a cheap cache update. Existing `GetStatus` reads the same projection source, so event and recovery paths cannot diverge semantically.

Alternative considered: call the application core from NimBLE callbacks. That creates a second state owner and timing/order ambiguity. Alternative considered: retain every tick. Only the latest complete snapshot matters; history uses the journal.

### 4. Refactor the core catalog from borrowed static data to fixed-capacity owned values

`focus-core` gains bounded owned preset identifiers and names, an owned catalog capacity of 13 entries, and a custom-entry marker. Firmware constructs the five existing built-ins first and appends at most eight persisted custom entries. `App`, `SessionState`, `AppSnapshot`, effects, settings restoration, and outcome snapshots clone bounded values where they currently copy borrowed values.

`App::replace_catalog` is legal only while Idle. It retains selection by identifier when possible and otherwise selects built-in `focus`. Starting a session still clones an immutable active preset, so later catalog changes cannot rewrite an outcome or active duration. Validation remains pure and rejects the entire combined catalog on any violation.

The protocol accepts duration milliseconds but the custom product contract permits whole minutes from 1 through 720. This keeps the mobile editor and physical display predictable while leaving built-in/internal duration representation unchanged.

Alternative considered: keep static core presets and implement custom entries only in firmware. The encoder and core session state would then disagree about catalog ownership, undermining host testing. Alternative considered: allocate strings and vectors. Fixed capacities are sufficient and keep RAM and failure behavior explicit.

### 5. Persist custom presets as a two-slot recoverable document

A new `focus_presets` NVS namespace stores two versioned, checksummed catalog blobs plus small redundant metadata identifying the committed revision and slot. A commit writes and verifies the inactive blob before advancing metadata. Boot validates both slots and metadata, selects the highest uniquely valid committed revision, and can recover a valid slot from missing/corrupt metadata. Ambiguity or no valid custom blob falls back to built-ins and reports degradation; it never prevents boot.

Revision zero means no committed custom catalog. Each successful durable replacement increments the safe bounded revision once. Catalog data is independent from selected settings and the session journal. The selected identifier is updated only after the catalog commit succeeds and only when the prior custom selection was removed.

Alternative considered: overwrite one NVS blob in place. Although NVS itself has protections, an explicit two-slot document gives deterministic power-loss tests and mirrors the already-proven journal recovery discipline. Alternative considered: store each preset under a separate key. Complete replacement and revision consistency would then require a wider transaction protocol.

### 6. Model physical confirmation as a small host-tested coordinator

A `CatalogUpdateCoordinator` adjacent to `App` owns at most one fixed-capacity pending proposal, its client proposal ID, expected revision, and monotonic 15-second deadline. The BLE request decoder validates bounds and sends a copied proposal through a bounded mailbox. The main task stages it only when `App` is Idle and no proposal is pending.

While pending, the runtime renders a dedicated confirmation model. A short encoder press produces a `CommitCatalog` effect instead of starting a session; a long press rejects; rotation is ignored; timeout, BLE disconnect, replacement proposal, or reboot discards the pending value. Firmware validates once more, commits storage, then calls `App::replace_catalog` and publishes both a catalog result event and a live snapshot. On persistence failure, it restores normal Idle presentation with the previous catalog.

The proposal response states only that confirmation is pending and returns proposal ID plus expiry. The app claims synchronization only after a success result and a catalog read at the committed revision. If the result event is missed, a recovery read determines whether the expected revision and contents committed.

Alternative considered: commit immediately from the phone. The development BLE service has no ownership provisioning, so nearby mutation would be silent and unsafe. Alternative considered: use iOS pairing as authorization now. Bonding alone does not define ownership/reset UX and would broaden this change substantially.

### 7. Use complete replacement and optimistic revision checks instead of preset CRUD commands

`GetPresetCatalog` returns revision plus the complete combined ordered catalog and built-in markers. `ProposePresetCatalog` contains the expected revision, client-generated proposal ID, and the complete desired custom list. The device validates size, identifiers, names, durations, order, and collisions before staging. A mismatch returns a typed conflict containing the current revision; Running, Paused, or Completed returns `Busy`.

Complete replacement is idempotent at the document level and makes deletion/reordering atomic. The app stores one local draft per stable device identity and its base revision in SQLite. A conflict reloads the device document but preserves the draft for user reconciliation; the app never silently last-writer-wins.

Alternative considered: individual create/update/delete commands. They need partial-failure recovery, ordering operations, more identifiers, and multi-command transactional semantics. Alternative considered: phone-authoritative presets loaded only while connected. That would violate standalone operation.

### 8. Add live and preset state to the existing application runtime, not screen-local BLE code

`DeviceTransport` gains a cancellable event subscription. `DeviceClient` decodes and orders events, exposes typed catalog operations, and triggers recovery reads. The runtime provider owns connection, latest authoritative snapshot, staleness, and preset-sync state. Screens consume application models and never import BLE objects.

Muninn's root timer view replaces reconnect-only status with the live device snapshot and explicit stale copy. A Presets route uses the already fixed application header, shows immutable built-ins and editable custom rows, persists drafts locally, and drives the propose/confirm/read-back workflow. Mock scenarios cover happy path, offline drafts, busy state, timeout, rejection, conflict, corruption fallback, missed events, and disconnect recovery with no controller.

Alternative considered: put subscriptions directly in the timer screen. Navigation and foreground changes would create multiple listeners and inconsistent recovery. Alternative considered: save drafts only in React state. Offline edits would disappear on app restart and be easy to mistake for synchronized data.

## Risks / Trade-offs

- [One-second events contend with command/response traffic or starve input] → Use a separate characteristic and sender, latest-value coalescing, fixed buffers, resource telemetry, and physical encoder/OLED/buzzer stress acceptance.
- [Owned preset values increase core RAM and cloning cost] → Bound the combined catalog at 13, measure exact structure/stack/heap deltas, and keep copies off the one-millisecond sampling path.
- [Power fails after blob write but before metadata commit] → Use inactive-slot writes, checksums, redundant metadata, and boot reconciliation tests; never expose success before durable commit.
- [A result notification is lost after the user confirms] → Treat catalog read-back as the recovery truth and keep proposals identifiable and idempotently reconcilable.
- [Physical confirmation temporarily changes encoder semantics] → Show an unambiguous OLED prompt, use a strict 15-second bound, reject on disconnect/reboot, and restore normal one-press start immediately afterward.
- [An unowned client repeatedly prompts the user] → Permit only one pending proposal, replace/reject deterministically, rate-limit staging diagnostics, and retain physical confirmation as the mutation gate. Full ownership security remains required before distribution.
- [Protocol 1.0 client encounters 1.1 firmware] → Negotiate minor/capabilities, preserve every 1.0 fixture and message, and never send events to a client that did not subscribe with the advertised capability.
- [Catalog corruption removes expected custom entries] → Fall back to built-ins, expose degradation, preserve journal/settings isolation, and allow a new confirmed replacement.

## Migration Plan

1. Extend the registry, Rust/TypeScript models, codecs, malformed-input suites, and byte fixtures to protocol 1.1 while proving all 1.0 fixtures remain unchanged.
2. Refactor `focus-core` to owned bounded presets and add catalog-replacement/confirmation state tests before changing firmware adapters; preserve the five built-ins and existing behavior tests.
3. Add two-slot catalog persistence and host power-loss/corruption tests in a new namespace; boot old NVS data as revision zero with the previous selected preset restored when valid.
4. Add firmware catalog mailboxes, confirmation presentation, event cache/sender, and resource diagnostics. Flash and verify standalone behavior before connecting the app.
5. Extend mock transport/client/runtime with event ordering and catalog workflows, then build the Presets and live-timer mobile surfaces against mock mode.
6. Connect the physical BLE event characteristic and catalog requests, install the development build, and run end-to-end confirmation, reboot, offline, conflict, disconnect, and active-session acceptance.
7. Record firmware size, heap/stack, notification cadence, encoder latency, watchdog/reset behavior, and NVS recovery. Restore production firmware and regular BLE app mode after diagnostics.

Rollback is additive: an older 1.0 app continues to use history/status reads and ignores unavailable 1.1 capabilities. Older firmware ignores the new mobile UI capability and leaves local drafts unsynchronized. Reflashing older firmware ignores the separate `focus_presets` namespace and retains the existing selected-settings and journal data.

## Open Questions

- Final OLED wording and typography for the 15-second confirmation prompt can be refined during physical usability testing without changing the confirmation semantics or task order.
