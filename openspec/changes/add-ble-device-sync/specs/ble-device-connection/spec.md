## Purpose

Defines optional BLE discovery and connection behavior for one nearby Focus Timer while preserving the physical device as a complete standalone timer.

## ADDED Requirements

### Requirement: Discoverable Focus Timer service
When BLE initialization succeeds, the device SHALL advertise an identifiable Focus Timer service without requiring a phone, account, network, or modification to the physical timer flow.

#### Scenario: Foreground discovery
- **GIVEN** the device is powered, not connected to another central, and BLE initialized successfully
- **WHEN** the iPhone performs a foreground scan for the Focus Timer service
- **THEN** the device appears as a connectable candidate without exposing a hardware MAC address as its product identity

#### Scenario: Radio initialization failure
- **GIVEN** BLE initialization fails during boot
- **WHEN** the firmware continues startup
- **THEN** the failure is reported diagnostically and the encoder, display, buzzer, persistence, and offline session lifecycle remain usable

### Requirement: Handshake-gated connection
A client SHALL complete the versioned protocol handshake before requesting status or journal data, and the application SHALL present incompatible devices as unsupported rather than partially connected.

#### Scenario: Successful connection
- **GIVEN** the app connects to an advertising compatible device
- **WHEN** service discovery and handshake complete
- **THEN** the app associates the connection with the stable device identifier and exposes the device as ready for synchronization

#### Scenario: Handshake timeout
- **GIVEN** a BLE link is established but the expected service, characteristics, or handshake response is unavailable within the documented timeout
- **WHEN** the timeout expires
- **THEN** the app closes the unusable connection and reports a retryable connection error

#### Scenario: Protocol incompatibility
- **GIVEN** a BLE link to a device with an incompatible protocol major version
- **WHEN** handshake reports the mismatch
- **THEN** the app disconnects or leaves the device read-disabled and shows the supported-version problem without attempting synchronization

### Requirement: Connection loss isolation
Connecting, disconnecting, timing out, reconnecting, or losing BLE notifications SHALL NOT pause, delay, cancel, duplicate, or complete a physical focus session.

#### Scenario: Disconnect during running session
- **GIVEN** a phone is connected while the device is Running
- **WHEN** the phone leaves range or the app disconnects
- **THEN** the device continues the same monotonic session and produces its normal physical completion behavior

#### Scenario: Reconnection after absence
- **GIVEN** a previously known device has accumulated journal entries while disconnected
- **WHEN** the app later reconnects and completes handshake
- **THEN** the connection becomes ready for catch-up synchronization from the app's last stored sequence

### Requirement: Explicit connection state
The mobile client SHALL expose distinguishable Bluetooth-unavailable, permission-denied, scanning, connecting, handshaking, ready, disconnected, incompatible, and retryable-error states.

#### Scenario: Bluetooth is disabled
- **GIVEN** Bluetooth is unavailable on the phone
- **WHEN** the user opens device setup or requests a scan
- **THEN** the app shows that Bluetooth must be enabled and does not display a false connected state

#### Scenario: Permission is denied
- **GIVEN** the user denies the required Bluetooth permission
- **WHEN** discovery is requested
- **THEN** the app explains that discovery cannot proceed and offers the platform-appropriate recovery path

### Requirement: Foreground catch-up is sufficient for MVP
The system SHALL guarantee synchronization attempts while the app is foregrounded and ready, but SHALL NOT require continuous background scanning or guaranteed background execution for correctness.

#### Scenario: App resumes in range
- **GIVEN** the operating system suspended the app and the device continued operating
- **WHEN** the user foregrounds the app in range of the known device
- **THEN** the app reconnects or offers reconnection and then requests all records after its last stored sequence

#### Scenario: No background execution granted
- **GIVEN** iOS does not wake the application while the timer completes sessions
- **WHEN** those sessions are durably appended by the device
- **THEN** they remain available for a later foreground synchronization
