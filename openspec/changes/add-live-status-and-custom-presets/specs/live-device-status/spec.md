## Purpose

Expose the physical timer's current authoritative state to a foreground companion app without making the phone part of countdown correctness or session control.

## ADDED Requirements

### Requirement: Device-authoritative live snapshot
The device SHALL expose a complete live snapshot containing a boot-scoped status epoch, a strictly increasing status revision, timer view state, selected or active preset snapshot, and remaining duration. The phone MUST treat the device snapshot as authoritative and MUST NOT expose remote start, pause, resume, cancel, or dismiss controls in this capability.

#### Scenario: Idle encoder selection appears on the phone
- **WHEN** the connected user rotates the encoder while the device is idle
- **THEN** the next accepted live snapshot identifies the newly selected preset and its full duration

#### Scenario: Physical lifecycle remains authoritative
- **WHEN** the user starts, pauses, resumes, cancels, completes, or dismisses a session using the encoder
- **THEN** the phone presents the resulting device state without originating a lifecycle command

#### Scenario: Device reboots while remembered
- **WHEN** the device reboots and publishes a new status epoch
- **THEN** the phone discards ordering assumptions from the prior epoch and accepts the new authoritative snapshot

### Requirement: Bounded foreground status delivery
After a compatible foreground connection, the client SHALL read one current snapshot and subscribe to live snapshots before declaring the view live. The device SHALL publish a new snapshot within 500 milliseconds of a meaningful state or preset transition and at least once every 1,500 milliseconds while Running.

#### Scenario: Initial connection has no race gap
- **WHEN** the app connects while the user is operating the device
- **THEN** the app subscribes and obtains a current snapshot before presenting a live state, without requiring a reconnect

#### Scenario: Running countdown remains visibly current
- **WHEN** the device remains Running with a foreground subscribed phone
- **THEN** the phone receives periodic authoritative remaining-duration corrections no more than 1,500 milliseconds apart

#### Scenario: Non-running state is quiet
- **WHEN** the device remains unchanged in Idle, Paused, or Completed state
- **THEN** it is not required to emit periodic duplicate snapshots

### Requirement: Ordered and recoverable observation
The client SHALL accept only a newer status revision within the same status epoch. A missing, malformed, duplicated, or out-of-order live snapshot MUST NOT mutate local history or device state, and the client SHALL recover by reading a fresh current snapshot after connection, foreground resume, detected revision gap, or explicit retry.

#### Scenario: Notification is missed
- **WHEN** the client observes a revision gap or returns to the foreground after notifications may have been missed
- **THEN** it reads and presents a fresh complete snapshot rather than reconstructing unknown intermediate transitions

#### Scenario: Duplicate notification arrives
- **WHEN** a live snapshot repeats an already accepted epoch and revision
- **THEN** the client ignores it without duplicating history or regressing the displayed timer

#### Scenario: Subscription fails after a valid connection
- **WHEN** the response handshake succeeds but live-status subscription cannot be established
- **THEN** the app reports that live updates are unavailable, retains local history, and offers a bounded retry or current-status refresh

### Requirement: Honest connected and stale presentation
The mobile app SHALL distinguish live connected state from a remembered but stale snapshot. It MAY interpolate display-only seconds between authoritative Running snapshots, but MUST reconcile on every accepted snapshot and MUST stop claiming live state when the BLE link or adapter becomes unavailable.

#### Scenario: Link drops during a running session
- **WHEN** BLE disconnects while the last snapshot was Running
- **THEN** the physical timer continues independently and the phone marks the retained snapshot stale instead of presenting a phone-owned authoritative countdown

#### Scenario: Bluetooth is disabled
- **WHEN** the user disables Bluetooth after a snapshot was received
- **THEN** the app preserves the last snapshot and local history, labels live status unavailable, and does not relabel the remembered device as unpaired

#### Scenario: Reconnection corrects interpolation
- **WHEN** a phone reconnects after locally interpolating a displayed countdown
- **THEN** the next device snapshot immediately corrects state and remaining duration without changing device behavior

### Requirement: Live transport failure isolation
Encoding, fragmentation, notification, subscription, radio, and client lifecycle failures MUST NOT block or delay monotonic timer transitions, physical input, OLED rendering, buzzer feedback, settings persistence, or journal appends beyond their existing acceptance bounds.

#### Scenario: Notification cannot be delivered
- **WHEN** the device produces a snapshot while no subscribed central can receive it
- **THEN** the device drops or coalesces delivery work and continues standalone operation without waiting for the phone

#### Scenario: Slow client falls behind
- **WHEN** status snapshots are produced faster than the BLE client consumes them
- **THEN** the transport retains at most the newest bounded snapshot and does not build an unbounded queue

#### Scenario: Status encoding fails
- **WHEN** a live snapshot cannot be encoded within protocol limits
- **THEN** the failure is reported diagnostically and the timer remains usable with its previous connectivity behavior
