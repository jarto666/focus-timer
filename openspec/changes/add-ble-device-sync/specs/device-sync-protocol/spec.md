## Purpose

Defines a compact, versioned, transport-independent contract that lets Rust firmware and TypeScript clients exchange device and session data predictably over BLE and future transports.

## ADDED Requirements

### Requirement: Versioned request and response contract
Every protocol request and response SHALL carry enough information to identify its protocol compatibility, message type, and request correlation, and the device SHALL return a structured error for unsupported valid requests.

#### Scenario: Compatible handshake
- **GIVEN** a client and device that support the same protocol major version
- **WHEN** the client sends a handshake request
- **THEN** the device returns its stable device identifier, firmware version, supported protocol version, and advertised capabilities correlated to that request

#### Scenario: Incompatible major version
- **GIVEN** a client requests a protocol major version the device cannot support
- **WHEN** the device decodes the handshake
- **THEN** it returns an explicit incompatible-version result containing the device-supported version and performs no requested state change

#### Scenario: Unknown request type
- **GIVEN** a syntactically valid envelope with an unsupported request type
- **WHEN** the device processes it
- **THEN** the device returns a correlated unsupported-message error and continues serving later valid requests

### Requirement: Deterministic cross-language representation
Rust and TypeScript implementations SHALL encode the same supported logical message into byte-identical canonical fixtures and SHALL decode every committed compatibility fixture.

#### Scenario: Golden request fixture
- **GIVEN** a committed handshake or session-page fixture
- **WHEN** Rust and TypeScript independently encode its logical value
- **THEN** both outputs match the committed bytes exactly

#### Scenario: Historical fixture remains readable
- **GIVEN** a fixture committed for a previously supported minor version of the current protocol major version
- **WHEN** the current Rust and TypeScript decoders read it
- **THEN** both produce the documented logical value or the documented forward-compatible default

### Requirement: Bounded and defensive decoding
The protocol SHALL define finite envelope, payload, page, and reassembled-message limits, and receivers SHALL reject malformed, truncated, duplicated, out-of-order, or oversized data without panicking or mutating timer state.

#### Scenario: Oversized message
- **GIVEN** fragments that declare or accumulate beyond the documented maximum logical message size
- **WHEN** a receiver processes them
- **THEN** it discards the partial message, reports a bounded protocol error, and remains able to receive a subsequent valid message

#### Scenario: Invalid payload
- **GIVEN** a complete envelope whose payload is malformed for its declared message type
- **WHEN** the device decodes it
- **THEN** the device returns or logs a decode error and preserves the current timer, settings, and journal state

### Requirement: Transport-independent logical messages
Logical protocol messages SHALL have the same meaning regardless of BLE fragmentation or a future byte-stream transport, and transport framing SHALL not appear in the domain or journal data model.

#### Scenario: Fragmented BLE transfer
- **GIVEN** a logical session-page response larger than the negotiated BLE packet payload
- **WHEN** it is sent over BLE
- **THEN** the client reassembles exactly one response with the original request correlation and logical contents

#### Scenario: Retransmitted read request
- **GIVEN** a client repeats a read-only status or session-page request after losing the response
- **WHEN** the device processes the repeated request
- **THEN** it returns a valid snapshot or page without duplicating or modifying any journal record

### Requirement: Minimal synchronization command surface
The first protocol version SHALL support handshake, current-status read, bounded journal-page read, and optional wall-clock-anchor update, and SHALL NOT expose remote start, pause, resume, cancel, preset mutation, Wi-Fi provisioning, or firmware update commands.

#### Scenario: Current status read
- **GIVEN** a connected compatible client
- **WHEN** it requests current status
- **THEN** the response reports the current view state, selected or active preset snapshot, remaining duration, journal bounds, and whether wall-clock time is known

#### Scenario: Clock anchor update
- **GIVEN** a connected compatible client supplies a valid current UTC time anchor
- **WHEN** the device accepts the anchor
- **THEN** it acknowledges the accepted anchor without starting, stopping, pausing, resuming, or otherwise changing the active focus session

#### Scenario: Remote-control request is unavailable
- **GIVEN** a connected client
- **WHEN** it attempts to use an unrecognized message as a remote timer-control command
- **THEN** the device returns an unsupported-message error and the timer remains controlled only by its physical interface
