## Purpose

Defines the first iPhone companion experience for connecting to a Focus Timer, synchronizing durable session outcomes, and reviewing local history without a cloud account.

## ADDED Requirements

### Requirement: Mock and physical device parity
The mobile application SHALL consume one device-client contract that can be backed by a deterministic mock device or by BLE, and both backends SHALL expose the same logical handshake, status, journal-page, error, and connection-state values.

#### Scenario: Develop without hardware
- **GIVEN** no ESP32 is connected
- **WHEN** the app runs in documented mock mode
- **THEN** the developer can exercise discovery, connection, empty history, paged history, duplicate pages, gaps, disconnects, and protocol errors without changing screen code

#### Scenario: Switch to BLE backend
- **GIVEN** the same mobile build supports the real-device configuration
- **WHEN** a compatible Focus Timer is selected
- **THEN** screens receive the same logical models through the BLE-backed device client

### Requirement: Single-device setup and reconnection
The application SHALL let the user discover and select one nearby compatible timer, remember its stable identity locally, and reconnect or offer reconnection on a later foreground launch.

#### Scenario: First successful setup
- **GIVEN** Bluetooth permission is granted and a compatible device is advertising
- **WHEN** the user selects it and handshake succeeds
- **THEN** the app stores its stable identity as the current device and begins synchronization

#### Scenario: Known device is unavailable
- **GIVEN** a current device was previously stored but is now powered off or out of range
- **WHEN** the app opens
- **THEN** locally stored history remains readable and the app shows the device as unavailable without deleting its identity or records

### Requirement: Local history synchronization and presentation
The application SHALL durably store synchronized records on the phone and present a simple session history that distinguishes completed, cancelled, timestamped, untimestamped, and incomplete-history records.

#### Scenario: Completed history is synchronized
- **GIVEN** the connected device contains records newer than the local cursor
- **WHEN** foreground synchronization finishes
- **THEN** the history shows the newly committed records in a deterministic order and retains them after the device disconnects

#### Scenario: Untimestamped record is shown honestly
- **GIVEN** a synchronized record has no UTC timestamp
- **WHEN** history is rendered
- **THEN** the app displays its preset, duration, and outcome without assigning a fabricated session date

#### Scenario: Retention gap is visible
- **GIVEN** the device reports that older records were evicted before the phone synchronized them
- **WHEN** the corresponding history is viewed
- **THEN** the app marks that device history as incomplete rather than silently presenting it as exhaustive

### Requirement: Connection and synchronization errors are recoverable
The application SHALL distinguish connection failures from synchronization and local-storage failures, preserve already committed history, and provide a retry path that does not require reinstalling or forgetting the device.

#### Scenario: Disconnect during synchronization
- **GIVEN** one or more pages were committed and more remain
- **WHEN** BLE disconnects
- **THEN** committed history remains visible, the sync is marked incomplete, and retry resumes from the last committed sequence

#### Scenario: Local database operation fails
- **GIVEN** a valid device page is received but cannot be committed locally
- **WHEN** the failure is reported
- **THEN** existing history remains intact, the cursor does not advance past uncommitted data, and the user can retry

### Requirement: Phone remains optional and read-oriented
The first mobile application SHALL NOT be required for a physical session and SHALL NOT expose controls for starting, pausing, resuming, cancelling, or dismissing a device session.

#### Scenario: App is absent
- **GIVEN** the application is closed, uninstalled, disconnected, or has never paired
- **WHEN** the user operates the encoder and button
- **THEN** the device completes the full offline session lifecycle and retains outcomes subject to journal capacity

#### Scenario: Running status is observed
- **GIVEN** the app reads status while the device is Running or Paused
- **WHEN** the status is presented
- **THEN** it is displayed as device-owned read-only state with no mobile session-control action

### Requirement: History is local by default
The application SHALL store device identity and session history locally on the phone and SHALL NOT require an account, upload records to a server, or expose them to web or desktop clients in this change.

#### Scenario: Phone has no internet
- **GIVEN** Bluetooth is available but internet access is absent
- **WHEN** the app connects and synchronizes
- **THEN** setup, status, synchronization, and local history continue to work

#### Scenario: Application data is removed
- **GIVEN** the user deletes the application or clears its local data
- **WHEN** it is installed or opened again
- **THEN** no cloud restore is attempted, and later device catch-up is limited to records still retained by the device journal
