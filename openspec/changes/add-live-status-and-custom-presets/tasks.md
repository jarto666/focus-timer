## 1. Extend the Versioned Protocol

- [x] 1.1 Assign protocol 1.1 capability, message-kind, event-envelope, GATT event-characteristic, field, error, and bound identifiers in `protocol/schema.md` without changing any 1.0 identifier or fixture.
- [x] 1.2 Add bounded Rust models for status epoch/revision, live-status events, combined catalog entries, catalog reads, complete replacement proposals, staged responses, and commit-result events.
- [x] 1.3 Implement constrained-CBOR Rust encoding/decoding and validation for every new 1.1 model, including zero request IDs only for registered device events.
- [x] 1.4 Mirror the 1.1 models, codecs, validation, and capability negotiation in `@focus-timer/device-protocol`.
- [x] 1.5 Commit byte-identical cross-language golden fixtures and malformed/boundary tests while proving all historical 1.0 fixtures remain unchanged and decodable.
- [x] 1.6 Extend BLE framing tests for independent response/event transfer sequences, default and negotiated payload fragmentation, duplicate frames, gaps, corruption, timeouts, and bounded recovery.

## 2. Refactor and Test the Pure Rust Preset Domain

- [x] 2.1 Replace borrowed compile-time preset values with bounded owned identifiers/names and an owned 13-entry catalog while retaining the five built-ins and `no_std` host compatibility.
- [x] 2.2 Update `App`, session state, snapshots, effects, outcomes, and selected-settings restoration to clone bounded preset values without changing existing lifecycle behavior.
- [x] 2.3 Add complete-catalog validation for custom count, identifier/name UTF-8 byte bounds, uniqueness across built-ins/customs, whole-minute 1..720-minute durations, built-in order, and total capacity.
- [x] 2.4 Implement Idle-only catalog replacement that retains selection by identifier or falls back to built-in `focus`, with active preset snapshots remaining immutable.
- [x] 2.5 Add a pure `CatalogUpdateCoordinator` for one pending proposal, monotonic 15-second expiry, short-press confirm, long-press reject, disconnect/replacement/reboot cancellation, and commit effects.
- [x] 2.6 Extend host behavior tests for legacy browsing/start flows, custom ordering/wrap, replacement selection rules, all confirmation outcomes, and rejection during Running, Paused, or Completed state.

## 3. Add Recoverable Device Catalog Persistence

- [x] 3.1 Define a versioned checksummed fixed-capacity custom-catalog document and redundant revision metadata in a new `focus_presets` namespace.
- [x] 3.2 Implement inactive-slot write, verification, metadata advance, and read-back semantics that expose success only after a complete durable commit.
- [x] 3.3 Implement boot reconciliation for first boot, one valid slot, corrupt metadata, interrupted commit, corrupt newest slot, ambiguous slots, and complete fallback to built-ins with degradation.
- [x] 3.4 Add host persistence tests for successful replacement/reboot, revision monotonicity, every power-loss boundary, corruption, maximum encoded catalog, selected-settings isolation, and journal isolation.
- [ ] 3.5 Add focused catalog clear/corrupt/fill diagnostics with safe documented commands and an unambiguous path back to production firmware.

## 4. Integrate Live Status and Confirmation in Firmware

- [x] 4.1 Add boot-scoped random status epoch and monotonically increasing revision to the immutable firmware status projection and compatible `GetStatus` response.
- [x] 4.2 Publish immediate full snapshots after meaningful selection/lifecycle/catalog transitions and one-second Running snapshots through a fixed latest-value cache.
- [x] 4.3 Add the notify-only GATT event characteristic, negotiated capability gating, independent event fragmentation, subscription lifecycle, and a non-blocking latest-value sender.
- [x] 4.4 Route validated catalog read/proposal requests through bounded mailboxes without granting BLE callbacks access to `App`, encoder GPIO, OLED, NVS, or session transitions.
- [x] 4.5 Integrate pending-catalog confirmation with encoder interception, 15-second monotonic timeout, disconnect cancellation, and explicit pending/success/rejected/expired/conflict/busy/storage-failure results.
- [x] 4.6 Add an OLED confirmation presentation that clearly shows an app-requested preset update and count while restoring normal one-press Idle start immediately after resolution.
- [ ] 4.7 Prove with host adapter tests that slow/unsubscribed clients coalesce snapshots, malformed events and notification failures recover, and radio/catalog failures cannot block timing, input, OLED, buzzer, settings, or journal work.
- [ ] 4.8 Build every firmware diagnostic and production feature combination, then record size and static resource deltas before physical flashing.

## 5. Extend the TypeScript Client and Deterministic Mock

- [x] 5.1 Extend `DeviceTransport` with one cancellable event subscription that remains independent of correlated request/response traffic and adapter availability observation.
- [x] 5.2 Implement `DeviceClient` event decoding, epoch/revision ordering, duplicate rejection, gap detection, recovery reads, catalog reads, proposals, result correlation, and read-back reconciliation.
- [ ] 5.3 Extend the deterministic mock with live transition/tick events, event loss/reordering/corruption, reboot epochs, bounded catalogs, physical-confirmation outcomes, storage failures, busy/conflict behavior, and disconnect cancellation.
- [ ] 5.4 Add client/mock tests for subscription-before-read startup, foreground/resume recovery, stale snapshots, interpolation correction, offline drafts, confirmation success/rejection/expiry, missed result recovery, and optimistic conflicts.

## 6. Build the Muninn Live Timer and Presets Experience

- [x] 6.1 Extend the application runtime provider with latest authoritative snapshot, live/stale status, foreground recovery, event subscription cleanup, and catalog synchronization state without exposing BLE objects to screens.
- [x] 6.2 Add a backward-compatible SQLite migration for one bounded custom draft and base revision per stable device identity, including restart/offline persistence and transactional replacement.
- [x] 6.3 Replace reconnect-only timer content with the device's live Idle/Running/Paused/Completed state, preset, remaining time, honest stale copy, and display-only monotonic interpolation corrected by snapshots.
- [x] 6.4 Add a Presets route under the fixed Muninn header with immutable built-ins and create/edit/reorder/delete controls for at most eight custom drafts.
- [x] 6.5 Implement validation and explicit unsynchronized, awaiting-device-confirmation, synchronized, busy, conflict, expired, rejected, unavailable, and storage-failure presentation states.
- [ ] 6.6 Add mobile model, persistence, runtime, navigation, accessibility, and screen tests in mock mode; pass formatting, lint, typecheck, unit, and production iOS build gates.

## 7. Physical End-to-End Acceptance

- [ ] 7.1 Flash protocol 1.1 firmware and verify existing standalone built-in selection, start/pause/resume/cancel/complete, settings restore, history sync, and a protocol 1.0 client remain functional.
- [ ] 7.2 Verify on the unlocked iPhone that encoder selection and every physical lifecycle transition appear live, Running corrections meet the 1,500-millisecond bound, and disconnect/reconnect/foreground resume converge to current device truth.
- [ ] 7.3 Create, edit, reorder, delete, confirm, reject, expire, conflict, and retry custom drafts on the phone; verify the committed catalog survives reboot and remains fully usable with the phone absent.
- [ ] 7.4 During active synchronization and one-second events, stress encoder rotation/presses while observing OLED, buzzer, session/journal exactness, heap/stack reserve, notification cadence, watchdogs, and resets.
- [ ] 7.5 Inject interrupted/corrupt catalog persistence, missed status/result events, BLE disconnect during confirmation, and radio failure; verify fallback/recovery plus unchanged standalone timer and journal behavior.
- [ ] 7.6 Record compact reproducible evidence, update development/acceptance docs and OpenSpec checkboxes only for observed results, restore production firmware/app mode, and run `check-all`, strict OpenSpec validation, and `git diff --check`.
