## Purpose

Define the fast, predictable selection of named focus durations that replaces repeated manual timer configuration.

## ADDED Requirements

### Requirement: Valid preset catalog
The device SHALL boot with a non-empty, firmware-defined preset catalog. Every preset MUST have a stable unique identifier, a non-blank display name, and a positive duration that the device can represent without overflow.

#### Scenario: Prototype catalog is available
- **WHEN** the prototype boots with its shipping firmware configuration
- **THEN** the catalog contains Deep Work (90 minutes), Focus (50 minutes), Pomodoro (25 minutes), Reading (45 minutes), and Quick Sprint (15 minutes)

#### Scenario: Invalid catalog entry is detected
- **WHEN** a firmware build supplies a duplicate identifier, blank name, zero duration, or unrepresentable duration
- **THEN** catalog validation fails before that entry can be used to start a session

### Requirement: Idle preset browsing
While idle, the device SHALL show exactly one selected preset with its name and duration. Each semantic left or right rotation SHALL move the selection by one catalog entry and SHALL wrap at the ends of the catalog.

#### Scenario: Rotate to the next preset
- **WHEN** the device is idle and receives one RotateRight event
- **THEN** it selects and displays the next preset in catalog order

#### Scenario: Wrap past the first preset
- **WHEN** the first preset is selected and the device receives one RotateLeft event
- **THEN** it selects and displays the last preset

### Requirement: One-press session start
While idle, one short press SHALL start a session using a snapshot of the currently displayed preset without opening a menu or requiring confirmation.

#### Scenario: Start the displayed preset
- **WHEN** the user short-presses the encoder while a preset is displayed in idle state
- **THEN** a running session begins with that preset's identifier, name, and full duration

#### Scenario: Meet the common-flow interaction target
- **WHEN** a user begins from the ready idle screen and selects one of the visible catalog presets
- **THEN** a bench usability test confirms the session can be started within five seconds without documentation

### Requirement: Active preset is stable
Rotation during a running, paused, or completed session MUST NOT change the active session's preset or duration.

#### Scenario: Rotation during a session
- **WHEN** the device receives rotation events while a session is running or paused
- **THEN** the active preset name, original duration, and remaining-time semantics remain unchanged

