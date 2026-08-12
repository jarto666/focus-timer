## Purpose

Define minimal local persistence that remembers the user's last preset while preserving safe boot and timer availability when storage is missing or corrupt.

## ADDED Requirements

### Requirement: Persist selected preset
The device SHALL persist the stable identifier of the most recently selected preset and SHALL restore it on the next boot when that identifier is still present in the firmware catalog.

#### Scenario: Reboot after changing selection
- **WHEN** the user selects a different preset, the save succeeds, and the device reboots
- **THEN** the device boots Idle with that preset selected

### Requirement: Safe default selection
The device SHALL have a deterministic firmware-defined default preset. Missing, unreadable, unsupported-version, or semantically invalid persisted settings MUST NOT prevent boot and SHALL result in the default preset being selected.

#### Scenario: First boot
- **WHEN** no settings record exists
- **THEN** the device boots Idle with the firmware-defined default preset

#### Scenario: Persisted preset no longer exists
- **WHEN** the stored preset identifier is absent from the current firmware catalog
- **THEN** the device selects the firmware-defined default and remains able to start a session

#### Scenario: Settings record is corrupt
- **WHEN** the settings record cannot be decoded or validated
- **THEN** the device selects the firmware-defined default, reports the fallback through development logging, and remains able to start a session

### Requirement: Save failure is non-blocking
A failure to persist a newly selected preset MUST NOT revert the in-memory selection or prevent a session from starting.

#### Scenario: Storage write fails
- **WHEN** the selected preset changes and the storage adapter returns a write error
- **THEN** the new selection remains active for the current boot, sessions can use it, and the failure is reported through development logging

### Requirement: Active session is not persisted
The MVP settings record SHALL NOT contain an active-session snapshot or attempt to restore Running, Paused, or Completed state across a reboot.

#### Scenario: Settings are saved during a session
- **WHEN** any settings write occurs while a session exists
- **THEN** the saved record contains no active-session timing or lifecycle state

