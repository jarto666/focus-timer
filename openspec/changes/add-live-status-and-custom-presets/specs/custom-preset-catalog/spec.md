## Purpose

Let a phone configure a small durable preset catalog that the physical timer can browse and run autonomously while protecting active sessions and requiring local approval.

## ADDED Requirements

### Requirement: Bounded combined preset catalog
The device SHALL expose its immutable built-in presets followed by zero to eight ordered custom presets. Every custom preset MUST have a stable unique identifier of 1 to 32 UTF-8 bytes, a non-blank name of 1 to 32 UTF-8 bytes, and a whole-minute duration from 1 through 720 minutes; identifiers MUST be unique across built-in and custom entries.

#### Scenario: Device has no custom presets
- **WHEN** the device has never accepted a custom catalog
- **THEN** encoder browsing and catalog reads expose the existing built-in presets in their existing order

#### Scenario: Custom presets are available offline
- **WHEN** a confirmed custom catalog has been committed and the phone disconnects
- **THEN** the encoder browses the built-ins followed by the custom entries in committed order and can start any of them without a phone

#### Scenario: Proposed catalog violates a bound
- **WHEN** a proposal contains too many entries, a duplicate identifier, an invalid name, a non-whole-minute duration, an out-of-range duration, or an identifier colliding with a built-in
- **THEN** the device rejects the complete proposal and retains the prior catalog and selection

### Requirement: Revisioned catalog reads
The device SHALL return the complete combined catalog, identify which entries are built-in, and include a monotonically increasing catalog revision that changes only after a successful custom-catalog commit.

#### Scenario: Phone opens preset management
- **WHEN** a compatible connected app opens the presets experience
- **THEN** it reads the current revision and complete catalog before allowing synchronization

#### Scenario: Catalog read is retried
- **WHEN** the same catalog read is repeated without an intervening commit
- **THEN** the device returns the same revision, ordering, identifiers, names, durations, and built-in markers

### Requirement: Complete replacement with optimistic concurrency
The app SHALL propose the complete desired custom list together with the revision it previously read. The device MUST reject a proposal whose expected revision is stale and MUST NOT partially apply any entry.

#### Scenario: Current revision is proposed
- **WHEN** a valid complete custom list references the current catalog revision while the device is Idle
- **THEN** the device stages exactly that list for physical confirmation without changing the committed catalog

#### Scenario: Another commit won the race
- **WHEN** a proposal references an older catalog revision
- **THEN** the device rejects it as a conflict and the app reloads the device catalog while preserving the user's unsynchronized draft for reconciliation

#### Scenario: Session is not idle
- **WHEN** a proposal arrives while the device is Running, Paused, or Completed
- **THEN** the device rejects it as busy and leaves the active session, committed catalog, and selection unchanged

### Requirement: Physical confirmation before mutation
A valid staged catalog replacement SHALL require a short encoder press on the device within 15 seconds before persistence begins. While confirmation is pending, the display SHALL identify the requested preset update and entry count; a long press, timeout, BLE disconnect, replacement proposal, or device reboot SHALL reject the pending proposal without changing the committed catalog.

#### Scenario: User confirms locally
- **WHEN** the display shows a pending valid replacement and the user short-presses within 15 seconds
- **THEN** the device begins one atomic commit of the staged complete custom list

#### Scenario: User rejects locally
- **WHEN** the user long-presses while confirmation is pending
- **THEN** the device rejects the proposal, restores the normal Idle presentation, and starts no session

#### Scenario: User ignores the proposal
- **WHEN** 15 seconds elapse without a confirming press
- **THEN** the device discards the staged data, restores normal Idle behavior, and reports an expired result to a still-connected app

#### Scenario: Phone disconnects before confirmation
- **WHEN** the proposing BLE connection ends before a confirming press
- **THEN** the device discards the pending proposal and the next encoder press retains its normal Idle start behavior

### Requirement: Atomic durable catalog commit
After physical confirmation, the device SHALL validate again, persist a versioned and checksummed complete custom catalog atomically, increment the revision, update the in-memory catalog, and report success only after the durable commit succeeds. Failure MUST retain the previous valid catalog and MUST NOT prevent standalone timer use.

#### Scenario: Commit succeeds
- **WHEN** the confirmed catalog is persisted successfully
- **THEN** one new revision becomes visible to reads, encoder browsing uses the new combined catalog, and the same revision survives reboot

#### Scenario: Storage fails during commit
- **WHEN** persistence fails before the new catalog is durably committed
- **THEN** the prior revision and complete prior catalog remain authoritative, normal timer operation resumes, and the app receives an explicit failure

#### Scenario: Stored custom catalog is corrupt at boot
- **WHEN** the persisted custom catalog cannot be decoded, verified, or semantically validated
- **THEN** the device boots with all built-ins usable, excludes the corrupt custom entries, reports catalog degradation, and permits a later confirmed replacement

### Requirement: Stable selection and active-session snapshot
Applying a catalog SHALL NOT alter a Running, Paused, or Completed session. If an Idle selected custom preset is removed by a successful commit, the device SHALL select the built-in Focus preset; otherwise it SHALL retain selection by stable identifier.

#### Scenario: Selected custom preset remains
- **WHEN** a successful replacement still contains the selected custom identifier
- **THEN** that preset remains selected with its newly committed name and duration

#### Scenario: Selected custom preset is removed
- **WHEN** a successful replacement removes the currently selected custom identifier
- **THEN** the built-in Focus preset becomes selected and is persisted as the normal selection

#### Scenario: Active preset data cannot be edited
- **WHEN** the app attempts catalog mutation during an active or completed session
- **THEN** the immutable active preset snapshot and lifecycle remain unchanged and the mutation is rejected

### Requirement: Mobile draft and synchronization states
The mobile app SHALL let the user create, reorder, edit, and delete custom preset drafts locally and SHALL distinguish unsaved draft, awaiting physical confirmation, synchronized, conflict, busy, expired, rejected, and persistence-failure states. Built-in entries SHALL be visible but not editable or deletable.

#### Scenario: User edits presets while device is unavailable
- **WHEN** the remembered device is offline and the user changes a local custom preset draft
- **THEN** the app preserves the draft locally, labels it unsynchronized, and does not claim that the device catalog changed

#### Scenario: Confirmation is required
- **WHEN** the device accepts a valid proposal for staging
- **THEN** the app instructs the user to confirm on the physical timer and does not label the draft synchronized until the committed revision is read back

#### Scenario: Built-in edit is attempted
- **WHEN** the user views a built-in preset in the app
- **THEN** its identifier, name, duration, and ordering are read-only
