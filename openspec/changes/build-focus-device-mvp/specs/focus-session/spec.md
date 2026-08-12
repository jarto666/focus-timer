## Purpose

Define a deterministic offline focus timer whose lifecycle and timing remain correct independently of connectivity and optional feedback hardware.

## ADDED Requirements

### Requirement: Explicit session lifecycle
The device SHALL represent session lifecycle as Idle, Running, Paused, or Completed and SHALL accept only the transitions defined by this specification.

#### Scenario: Start from idle
- **WHEN** the device is idle and receives a short press for a valid selected preset
- **THEN** it enters Running with remaining time equal to the preset duration

#### Scenario: Pause a running session
- **WHEN** the device is Running and receives a short press
- **THEN** it enters Paused and preserves the remaining time at the pause instant

#### Scenario: Resume a paused session
- **WHEN** the device is Paused and receives a short press
- **THEN** it enters Running and continues from the preserved remaining time

#### Scenario: Cancel without confirmation
- **WHEN** the device is Running or Paused and receives a semantic long press
- **THEN** it cancels the session and returns to Idle without a second confirmation step

#### Scenario: Dismiss completion
- **WHEN** the device is Completed and receives a short press
- **THEN** it returns to Idle with the previously selected preset still selected

### Requirement: Accurate monotonic countdown
While Running, the device SHALL derive remaining time from elapsed monotonic time rather than from a decrement-per-render counter. Remaining time MUST NOT increase while continuously running, and paused time MUST NOT reduce it.

#### Scenario: Running time elapses
- **WHEN** one second of monotonic time elapses while a session is Running
- **THEN** the displayed whole-second remaining value decreases accordingly on the next scheduled refresh

#### Scenario: Time passes while paused
- **WHEN** any amount of time elapses while a session is Paused
- **THEN** its remaining duration is unchanged

#### Scenario: Resume after pause
- **WHEN** a paused session resumes after a pause of arbitrary length
- **THEN** its completion time is based only on the remaining active duration and excludes the paused interval

#### Scenario: Prototype accuracy gate
- **WHEN** an uninterrupted session of up to 90 minutes is measured against a reference clock on the target board
- **THEN** completion occurs within one second of the configured active duration

### Requirement: Completion occurs once
When remaining time reaches zero, the device SHALL enter Completed exactly once and SHALL clamp the reported remaining time to zero.

#### Scenario: Countdown reaches zero
- **WHEN** monotonic elapsed active time equals or exceeds the session duration
- **THEN** the device enters Completed, reports zero remaining, and emits one completion event

#### Scenario: Later timer refreshes
- **WHEN** refresh events continue after the device has entered Completed
- **THEN** no additional completion event is emitted for that session

### Requirement: Offline operation
Starting, pausing, resuming, cancelling, timing, and completing a session MUST NOT require Wi-Fi, Bluetooth, a computer, a phone, cloud services, wall-clock time, or internet access.

#### Scenario: No connectivity exists
- **WHEN** the device has power and required local timer hardware but no network or companion connection
- **THEN** the complete session lifecycle remains available

### Requirement: Power-loss behavior
The MVP SHALL treat an interrupted active session as ended and SHALL boot back into Idle rather than attempting to reconstruct elapsed time without a trusted power-independent clock.

#### Scenario: Reboot during a running session
- **WHEN** power is lost during a session and the device later boots
- **THEN** no session is active and the device displays Idle with a valid selected preset

