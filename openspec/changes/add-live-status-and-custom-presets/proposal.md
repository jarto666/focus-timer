## Why

The phone can already reconnect to the timer and import completed history, but it does not show what the device is doing now and cannot configure the presets used without the phone. Live device state and a device-resident custom preset catalog are the next useful product slice because they make the companion app relevant before, during, and between sessions without weakening standalone operation.

## What Changes

- Stream authoritative timer snapshots from the ESP32-C3 to a connected foreground phone so encoder selection, start, pause, resume, cancel, completion, dismissal, and remaining time appear without reconnecting.
- Recover from missed notifications by reading a fresh snapshot after connection, foreground resume, or subscription failure; the device remains the sole countdown and session-state authority.
- Add a bounded custom preset catalog that is persisted on the device and remains usable while the phone is absent.
- Let the iPhone app list built-in and custom presets, create/edit/delete custom entries, and synchronize a complete revisioned custom catalog to the timer.
- Keep built-in presets immutable, keep an active session's preset snapshot immutable, and reject catalog mutation while a session is running, paused, or completed.
- Require a short physical encoder confirmation before the device commits an app-proposed catalog replacement, with explicit timeout/rejection/conflict results.
- Extend the versioned Rust/TypeScript protocol and deterministic mock with live-status notifications plus catalog read/propose/confirm outcomes.
- Add a native Presets experience and live timer presentation in Muninn while preserving offline history and remembered-device behavior.
- Explicitly defer remote start/pause/resume/cancel/dismiss, phone-owned countdown truth, background-always-on BLE, multiple owners, cloud sync, Wi-Fi, production bonding/provisioning, OTA, and enclosure/display hardware changes.

Assumptions requiring implementation-time validation:

- A one-second live snapshot cadence while running, plus immediate transition snapshots, fits the measured BLE/heap budget without affecting encoder sampling, OLED, buzzer, NVS, or monotonic timing.
- Eight custom presets with bounded identifiers, names, and durations fit the current NVS partition and OLED layout; the exact encoded size and write behavior must be measured on the connected board.
- Physical confirmation is sufficient authorization for this personal prototype. Production ownership, encryption, reset, and multi-phone policy remain a separate security change.

## Capabilities

### New Capabilities

- `live-device-status`: Publish, subscribe to, recover, and present the device-authoritative current timer snapshot over foreground BLE.
- `custom-preset-catalog`: Persist a bounded revisioned custom preset catalog on the device and manage it from the phone through physically confirmed replacement.

### Modified Capabilities

None. The earlier BLE and offline-MVP changes have not yet been archived into main specs; this change adds narrowly scoped capabilities without redefining their existing contracts.

## Impact

- Extends `focus-protocol` and `@focus-timer/device-protocol` message models, numeric schema registry, compatibility fixtures, framing tests, and structured errors.
- Extends the firmware BLE adapter with a separate status-notification path and a bounded pending-catalog confirmation flow, while retaining single ownership of `focus_core::App`.
- Adds versioned, checksummed custom-catalog persistence and catalog-aware preset selection to `focus-core`/firmware adapters.
- Extends `device-client`, `mock-device`, and the React Native application runtime with subscriptions, recovery reads, catalog synchronization, and conflict/error states.
- Adds mobile screens/components and local draft persistence for custom preset management; no account, server, or network dependency is introduced.
- Creates new cross-language and physical acceptance obligations for notification ordering, missed-event recovery, active-session immutability, NVS recovery, physical confirmation, resource use, and standalone operation.
