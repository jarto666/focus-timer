## Purpose

Defines durable, bounded session outcome records and idempotent catch-up synchronization so a phone can reconstruct device history after being offline or absent.

## ADDED Requirements

### Requirement: Durable session outcome records
The device SHALL append exactly one journal record when an active session first completes or is cancelled, and SHALL append no record for selection, pause, resume, completion dismissal, or ignored input.

#### Scenario: Completed session is recorded once
- **GIVEN** a Running session reaches zero
- **WHEN** the core commits its first transition to Completed and later receives refreshes or dismissal input
- **THEN** the journal contains exactly one Completed record for that session

#### Scenario: Cancelled running or paused session is recorded once
- **GIVEN** a Running or Paused session
- **WHEN** the user performs the accepted physical cancellation gesture
- **THEN** the journal contains exactly one Cancelled record with the accumulated active focus duration and the device returns to Idle

#### Scenario: Pause and resume do not create records
- **GIVEN** an active session is paused and resumed one or more times
- **WHEN** the journal is read before completion or cancellation
- **THEN** it contains no outcome record for those intermediate transitions

### Requirement: Stable identity and monotonic journal position
Each device SHALL maintain a stable product identity and journal epoch, assign every appended record a strictly increasing sequence within that epoch, preserve the position after reboot and eviction, and create a new epoch before any recovery that could otherwise reuse a sequence.

#### Scenario: Sequence survives reboot
- **GIVEN** the latest persisted record in journal epoch E has sequence N
- **WHEN** the device reboots and later appends another outcome
- **THEN** the new record remains in epoch E with a sequence greater than N

#### Scenario: Separate devices use independent sequences
- **GIVEN** two devices both contain journal epoch 1 and record sequence 1
- **WHEN** the phone stores both records
- **THEN** they remain distinct because local identity includes stable device identifier, journal epoch, and sequence

#### Scenario: Irrecoverable journal reset
- **GIVEN** the retained journal cannot be recovered without risking sequence reuse
- **WHEN** the device initializes a clean journal
- **THEN** it preserves the stable device identifier, selects a new journal epoch, and does not present the new journal as a continuation of the old epoch

### Requirement: Self-contained record data
A journal record SHALL contain sequence, preset identifier, preset name snapshot, planned duration, accumulated active focus duration, outcome, and optional start/end UTC timestamps without depending on the current preset catalog.

#### Scenario: Preset later changes
- **GIVEN** a record was created from a preset snapshot
- **WHEN** the device's current preset catalog later changes
- **THEN** the historical record retains the identifier, name, and planned duration captured for that session

#### Scenario: Clock is unknown
- **GIVEN** no valid wall-clock anchor exists when a session starts or ends
- **WHEN** its outcome is journaled
- **THEN** sequence, preset data, durations, and outcome are preserved while unavailable UTC timestamps are explicitly absent

### Requirement: Bounded paged retrieval with gap reporting
The journal SHALL have a documented finite capacity, return records in ascending sequence through bounded pages, expose its oldest and latest retained sequences, and explicitly report when a requested cursor predates retained history.

#### Scenario: Normal catch-up page
- **GIVEN** retained records exist after client sequence N
- **WHEN** the client requests a page after N within the retained range
- **THEN** the device returns the next ascending bounded page plus a continuation cursor and current journal bounds

#### Scenario: Client is older than retained history
- **GIVEN** the device evicted records newer than the client's last stored sequence because capacity was reached
- **WHEN** the client requests records after its stale sequence
- **THEN** the response explicitly reports a history gap and begins at the oldest retained record rather than pretending the history is complete

#### Scenario: Client is current
- **GIVEN** the client's last stored sequence equals the latest device sequence
- **WHEN** it requests the next page
- **THEN** the device returns an empty successful page with unchanged journal bounds

### Requirement: Idempotent local synchronization
The mobile store SHALL make repeated or interrupted synchronization safe by uniquely identifying imported records by stable device identifier, journal epoch, and sequence and advancing its cursor only for durably committed records.

#### Scenario: Duplicate page arrives
- **GIVEN** a page was committed locally but its acknowledgement or connection was lost
- **WHEN** the same page is received again
- **THEN** no duplicate history entries are created and the stored records remain unchanged

#### Scenario: Synchronization stops mid-page
- **GIVEN** a page is received but local persistence fails before the transaction commits
- **WHEN** synchronization is retried
- **THEN** the uncommitted cursor is not treated as complete and the records are requested or processed again

### Requirement: Journal failure does not break timing
Journal initialization, append, read, or persistence failure SHALL be reported as synchronization degradation and SHALL NOT prevent boot, physical control, monotonic countdown, completion feedback, or safe return to Idle.

#### Scenario: Append persistence fails
- **GIVEN** a session outcome is committed but its durable journal append fails
- **WHEN** the firmware handles the failure
- **THEN** the timer completes or cancels normally, reports the journal degradation, and does not fabricate a successfully persisted record

#### Scenario: Journal is unreadable at boot
- **GIVEN** journal metadata or records are corrupt or unsupported
- **WHEN** the device boots
- **THEN** it isolates or resets the journal according to the documented recovery policy while preserving standalone timer usability and avoiding reuse of an already established device identity
