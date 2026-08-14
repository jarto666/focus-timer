# Focus Timer device synchronization protocol registry

Status: **protocol 1.0 registry**  
Wire encoding: **deterministic constrained CBOR**  
BLE frame version: **1**

This file is the language-independent source of truth for the Focus Timer synchronization
protocol. Rust and TypeScript codecs must implement this registry explicitly and must produce the
same bytes for every committed fixture.

The protocol is deliberately read-oriented. It has no command for starting, pausing, resuming,
cancelling, dismissing, or editing a timer; no Wi-Fi provisioning; and no firmware update path.
`SetClockAnchorRequest` may update only the volatile UTC anchor used for optional history
timestamps. The physical device remains the sole owner of timer state.

## 1. BLE GATT registry

All UUIDs below are private 128-bit UUIDs. They are not Bluetooth SIG-assigned identifiers.

| Role                    | UUID                                   | GATT properties                          |
| ----------------------- | -------------------------------------- | ---------------------------------------- |
| Focus Timer service     | `1cf47046-2e37-4642-a30e-df24879f994f` | Primary service; included in advertising |
| Command characteristic  | `65ecdf0d-cde0-4543-a62b-c166c3341319` | Write with response                      |
| Response characteristic | `2c4e304b-2581-481a-8646-89122d760711` | Notify                                   |

The client must subscribe to the response characteristic before writing a command. The device
accepts one connected central and one request awaiting a response. The advertising or scan-response
local name is `FocusTimer`; it is only a product hint. Advertising must not contain the stable
device identifier, journal epoch, session data, or hardware MAC address as product identity.

## 2. Compatibility policy

The current protocol version is `1.0`.

- The **major** version changes when an existing identifier, field meaning, required field, value
  representation, or compatibility rule changes incompatibly.
- The **minor** version changes only for additive behavior within the same major: a new optional
  field, capability-gated message kind, enum value, or error code.
- An assigned numeric identifier must never be reused for a different meaning within a major
  version. Removing a feature reserves its identifiers permanently.
- Peers with the same major negotiate `min(client_minor, device_minor)` during the hello exchange.
  Requests after hello use that negotiated version.
- A decoder must ignore unknown map fields after validating their CBOR values when the major is
  compatible. It must not infer support for an unknown capability, enum value, or message kind.
- A client must use only capabilities it recognizes and that the device returned in hello.
- An unknown valid request kind produces `UnsupportedMessage`; it never falls through to timer
  control or another operation.
- A major mismatch permits only a correlated `UnsupportedProtocolVersion` error. That error is
  encoded with the device's supported version so the pre-handshake client can report the mismatch.

Committed golden fixtures are compatibility artifacts. Changing their bytes or reinterpretation
requires an explicit version transition; reformatting a fixture in place is not allowed.

## 3. Constrained CBOR profile

Every logical request and response is one CBOR envelope. Encoders and decoders implement only this
profile:

1. Maps, arrays, byte strings, and text strings use definite lengths.
2. Integers and lengths use the shortest valid CBOR representation.
3. Every map key is an unsigned integer and keys are emitted in ascending numeric order.
4. Map keys are unique. Duplicate keys are a decode error even when the values are equal.
5. Supported values are unsigned integers, booleans, byte strings, well-formed UTF-8 strings,
   arrays, and maps.
6. Negative integers, floats, null/undefined, tags, indefinite values, and other simple values are
   forbidden.
7. Text limits are measured in encoded UTF-8 bytes, not Unicode scalar or UI character count.
8. Required fields must be present. Optional unavailable values are represented by omitting their
   field; they are never represented by `null`, zero, an empty string, or a fabricated value.
9. A known message with an unknown enum value or invalid field combination is `InvalidField`.
10. A decoder validates the complete envelope and payload before invoking application behavior.

These rules are a deliberately smaller profile than general CBOR. Generic object serialization is
not a conforming encoder.

## 4. Global limits and scalar conventions

| Name                         |  Value | Meaning                                                |
| ---------------------------- | -----: | ------------------------------------------------------ |
| `PROTOCOL_MAJOR`             |    `1` | Current major version                                  |
| `PROTOCOL_MINOR`             |    `0` | Current minor version                                  |
| `MAX_LOGICAL_MESSAGE_BYTES`  | `2048` | Complete CBOR envelope, before BLE framing             |
| `MAX_RECORDS_PER_PAGE`       |    `8` | Requested and returned session records                 |
| `MAX_CAPABILITIES`           |    `8` | Unique capability identifiers in hello                 |
| `MAX_TEXT_BYTES`             |   `32` | Default text-string bound                              |
| `MAX_PRODUCT_NAME_BYTES`     |   `24` | Hello product name                                     |
| `MAX_FIRMWARE_VERSION_BYTES` |   `32` | Hello firmware version                                 |
| `MAX_PRESET_ID_BYTES`        |   `32` | Preset identifier snapshot                             |
| `MAX_PRESET_NAME_BYTES`      |   `32` | Preset display-name snapshot                           |
| `MAX_MAP_ENTRIES`            |   `16` | Defensive global map bound                             |
| `MAX_ARRAY_ITEMS`            |    `8` | Defensive bound unless a schema table is stricter      |
| `MAX_NESTING_DEPTH`          |    `6` | Container nesting including envelope and record preset |
| `MAX_IN_FLIGHT_REQUESTS`     |    `1` | Per connection                                         |

Scalar conventions:

- `request_id` is an unsigned 32-bit integer in `1..=0xffff_ffff`. Zero is reserved.
- Durations are unsigned milliseconds in `0..=0xffff_ffff`.
- Journal sequences are in `1..=9_007_199_254_740_991` (`Number.MAX_SAFE_INTEGER`). Zero is
  reserved as the cursor before the first sequence. A device must create a new journal epoch before
  it could exceed the bound.
- UTC and device-monotonic millisecond values are in
  `0..=9_007_199_254_740_991`. No signed epoch values are supported in protocol 1.
- A stable `device_id` is exactly 16 opaque random bytes. It is not a BLE peripheral identifier.
- A `journal_epoch` is exactly 8 opaque random bytes. It is not a counter with cross-device
  meaning.
- Capability arrays are unique and ascending. Session records are ascending by sequence.

## 5. Envelope registry

Every message is a map with exactly the four scalar fields plus a payload map.

| Key | Name             | Type | Required | Rule                                      |
| --: | ---------------- | ---- | -------- | ----------------------------------------- |
| `0` | `protocol_major` | uint | yes      | Version used to encode this envelope      |
| `1` | `protocol_minor` | uint | yes      | Version used to encode this envelope      |
| `2` | `request_id`     | uint | yes      | Correlates one request and one response   |
| `3` | `message_kind`   | uint | yes      | Registered below                          |
| `4` | `payload`        | map  | yes      | Message-specific; empty requests use `{}` |

Every response copies the request's non-zero `request_id`. Except for the pre-handshake major
mismatch described above, a response uses the negotiated protocol version. A client ignores a
response whose request ID does not match its sole in-flight request and then applies its bounded
request timeout.

### Message-kind identifiers

|    ID | Name                     | Direction       | Required capability |
| ----: | ------------------------ | --------------- | ------------------- |
|   `1` | `HelloRequest`           | client → device | baseline            |
|   `2` | `HelloResponse`          | device → client | baseline            |
|   `3` | `GetStatusRequest`       | client → device | `ReadStatus`        |
|   `4` | `GetStatusResponse`      | device → client | `ReadStatus`        |
|   `5` | `GetSessionPageRequest`  | client → device | `ReadSessionPages`  |
|   `6` | `GetSessionPageResponse` | device → client | `ReadSessionPages`  |
|   `7` | `SetClockAnchorRequest`  | client → device | `SetClockAnchor`    |
|   `8` | `SetClockAnchorResponse` | device → client | `SetClockAnchor`    |
| `255` | `ErrorResponse`          | device → client | baseline            |

`0`, `9..254`, and values above `255` are reserved for future registry revisions.

## 6. Shared nested registries

### Capability identifiers

|  ID | Name               | Meaning                                     |
| --: | ------------------ | ------------------------------------------- |
| `1` | `ReadStatus`       | Read the current immutable timer projection |
| `2` | `ReadSessionPages` | Read stateless bounded journal pages        |
| `3` | `SetClockAnchor`   | Update the volatile UTC anchor only         |

### Preset snapshot map

| Key | Name                  | Type | Required | Rule                       |
| --: | --------------------- | ---- | -------- | -------------------------- |
| `0` | `preset_id`           | text | yes      | `1..MAX_PRESET_ID_BYTES`   |
| `1` | `preset_name`         | text | yes      | `1..MAX_PRESET_NAME_BYTES` |
| `2` | `planned_duration_ms` | uint | yes      | Duration bound             |

### Timer view-state enum

| Value | Name        |
| ----: | ----------- |
|   `0` | `Idle`      |
|   `1` | `Running`   |
|   `2` | `Paused`    |
|   `3` | `Completed` |

### Journal-health enum

| Value | Name          |
| ----: | ------------- |
|   `0` | `Healthy`     |
|   `1` | `Degraded`    |
|   `2` | `Unavailable` |

### Session-outcome enum

| Value | Name        |
| ----: | ----------- |
|   `0` | `Completed` |
|   `1` | `Cancelled` |

## 7. Message payload registry

Fields not marked optional are required. Empty request payloads are definite-length empty maps.

### `HelloRequest` (`1`)

Payload: `{}`. The envelope version is the client's highest supported version.

### `HelloResponse` (`2`)

| Key | Name               | Type        | Rule                                      |
| --: | ------------------ | ----------- | ----------------------------------------- |
| `0` | `device_id`        | bytes       | Exactly 16 bytes; stable product identity |
| `1` | `product_name`     | text        | `1..MAX_PRODUCT_NAME_BYTES`               |
| `2` | `firmware_version` | text        | `1..MAX_FIRMWARE_VERSION_BYTES`           |
| `3` | `supported_major`  | uint        | Device's highest supported major          |
| `4` | `supported_minor`  | uint        | Device's highest minor for that major     |
| `5` | `capabilities`     | array<uint> | Unique ascending IDs; maximum 8           |

After a compatible response, both peers use the minimum supported minor and the intersection of
known advertised capabilities. No request other than hello is valid before this point.

### `GetStatusRequest` (`3`)

Payload: `{}`.

### `GetStatusResponse` (`4`)

| Key | Name                    | Type | Rule                                       |
| --: | ----------------------- | ---- | ------------------------------------------ |
| `0` | `view_state`            | uint | Timer view-state enum                      |
| `1` | `preset`                | map  | Selected or active preset snapshot         |
| `2` | `remaining_duration_ms` | uint | Duration bound; zero when completed        |
| `3` | `journal`               | map  | Journal status map below                   |
| `4` | `clock_known`           | bool | Whether a valid volatile UTC anchor exists |

Journal status map:

| Key | Name              | Type  | Required | Rule                                 |
| --: | ----------------- | ----- | -------- | ------------------------------------ |
| `0` | `journal_epoch`   | bytes | yes      | Exactly 8 bytes                      |
| `1` | `oldest_sequence` | uint  | no       | Omitted when no records are retained |
| `2` | `latest_sequence` | uint  | no       | Omitted when no records are retained |
| `3` | `health`          | uint  | yes      | Journal-health enum                  |

`oldest_sequence` and `latest_sequence` are either both present with
`oldest_sequence <= latest_sequence`, or both absent.

### `GetSessionPageRequest` (`5`)

| Key | Name             | Type  | Required | Rule                                                 |
| --: | ---------------- | ----- | -------- | ---------------------------------------------------- |
| `0` | `journal_epoch`  | bytes | no       | Exactly 8 bytes when the client has a cursor         |
| `1` | `after_sequence` | uint  | yes      | Zero for the first page, otherwise a stored sequence |
| `2` | `limit`          | uint  | yes      | `1..=MAX_RECORDS_PER_PAGE`                           |

When `journal_epoch` is absent, `after_sequence` must be zero. Requests are stateless and
read-only; retrying the same tuple cannot advance or mutate device state.

### `GetSessionPageResponse` (`6`)

| Key | Name                          | Type       | Required | Rule                                                      |
| --: | ----------------------------- | ---------- | -------- | --------------------------------------------------------- |
| `0` | `journal_epoch`               | bytes      | yes      | Current 8-byte device epoch                               |
| `1` | `oldest_sequence`             | uint       | no       | Omitted only for an empty journal                         |
| `2` | `latest_sequence`             | uint       | no       | Omitted only for an empty journal                         |
| `3` | `gap`                         | bool       | yes      | Requested cursor predates retained data in the same epoch |
| `4` | `epoch_changed`               | bool       | yes      | Requested epoch differs from the current epoch            |
| `5` | `records`                     | array<map> | yes      | Ascending, unique, at most requested limit and 8          |
| `6` | `continuation_after_sequence` | uint       | no       | Last returned sequence when another page remains          |

For an empty journal, bounds and continuation are absent and records is empty. `gap` and
`epoch_changed` describe different conditions and cannot both be true. When `epoch_changed` is true,
the response begins a new epoch segment at its oldest retained record. When `gap` is true, it begins
at the oldest retained record in the requested epoch. An empty current page uses unchanged bounds,
an empty records array, and no continuation.

Session record map:

| Key | Name                 | Type | Required | Rule                                          |
| --: | -------------------- | ---- | -------- | --------------------------------------------- |
| `0` | `sequence`           | uint | yes      | Strictly increasing within the page and epoch |
| `1` | `preset`             | map  | yes      | Immutable preset snapshot                     |
| `2` | `active_duration_ms` | uint | yes      | `<= planned_duration_ms`                      |
| `3` | `outcome`            | uint | yes      | Session-outcome enum                          |
| `4` | `started_at_utc_ms`  | uint | no       | Omit when not provable from a valid anchor    |
| `5` | `ended_at_utc_ms`    | uint | no       | Omit when not provable from a valid anchor    |

Optional timestamps are independent. If both exist, `started_at_utc_ms <= ended_at_utc_ms`.
Synchronization or receipt time must never be substituted for either field.

### `SetClockAnchorRequest` (`7`)

| Key | Name     | Type | Rule                                                   |
| --: | -------- | ---- | ------------------------------------------------------ |
| `0` | `utc_ms` | uint | Current UTC milliseconds within the safe-integer bound |

On acceptance the device pairs this UTC value with its monotonic millisecond value at receipt. It
does not persist the anchor across a cold boot and does not modify monotonic session accounting.

### `SetClockAnchorResponse` (`8`)

| Key | Name                             | Type | Rule                                  |
| --: | -------------------------------- | ---- | ------------------------------------- |
| `0` | `accepted_utc_ms`                | uint | Accepted request value                |
| `1` | `device_monotonic_ms_at_receipt` | uint | Device value captured with the anchor |

### `ErrorResponse` (`255`)

| Key | Name                  | Type | Required | Rule                                       |
| --: | --------------------- | ---- | -------- | ------------------------------------------ |
| `0` | `code`                | uint | yes      | Error-code registry below                  |
| `1` | `failed_message_kind` | uint | no       | Present when the request kind was readable |
| `2` | `field_id`            | uint | no       | Present for a known invalid payload field  |
| `3` | `supported_major`     | uint | no       | Required for `UnsupportedProtocolVersion`  |
| `4` | `supported_minor`     | uint | no       | Required for `UnsupportedProtocolVersion`  |

Human-readable error strings are deliberately absent from the wire. Applications map stable codes
to localized copy. An envelope too malformed to recover a valid non-zero request ID may be dropped
and logged without a response.

## 8. Error-code registry

| Code | Name                         | Meaning                                                         |
| ---: | ---------------------------- | --------------------------------------------------------------- |
|  `1` | `MalformedEnvelope`          | Envelope violates CBOR, required-field, or global-limit rules   |
|  `2` | `UnsupportedProtocolVersion` | Protocol major is incompatible                                  |
|  `3` | `UnsupportedMessage`         | Valid envelope uses an unregistered or unavailable request kind |
|  `4` | `MalformedPayload`           | Payload is not the registered map shape                         |
|  `5` | `InvalidField`               | Known field value or field combination is invalid               |
|  `6` | `LimitExceeded`              | Declared page or value exceeds a protocol bound                 |
|  `7` | `HandshakeRequired`          | Non-hello request arrived before compatible hello               |
|  `8` | `Busy`                       | Another request or bounded device operation is active           |
|  `9` | `JournalUnavailable`         | Journal cannot currently serve a page                           |
| `10` | `InternalError`              | Bounded operation failed without changing timer state           |

`0` and values above `10` are reserved. Transport framing failures are not logical error responses:
the receiver resets reassembly, records a bounded diagnostic, and remains available for a later
valid transfer.

## 9. BLE frame format

Logical CBOR envelopes are fragmented only at the BLE transport boundary. Every GATT command write
or response notification contains a 12-byte header followed by a non-empty fragment payload.
Multi-byte integers are unsigned **big-endian**.

| Offset | Size | Name               | Rule                                               |
| -----: | ---: | ------------------ | -------------------------------------------------- |
|    `0` |    1 | `frame_version`    | `1`                                                |
|    `1` |    1 | `flags`            | Bit 0 `START`, bit 1 `END`; bits 2..7 must be zero |
|    `2` |    2 | `transfer_id`      | Non-zero client-selected ID                        |
|    `4` |    2 | `total_length`     | Complete logical size in `1..=2048`                |
|    `6` |    2 | `fragment_offset`  | Zero-based byte offset into the logical message    |
|    `8` |    4 | `message_crc32`    | CRC-32/ISO-HDLC of the complete logical CBOR bytes |
|   `12` | rest | `fragment_payload` | Non-empty contiguous logical bytes                 |

CRC-32/ISO-HDLC parameters are: reflected polynomial `0xedb88320`, initial value `0xffffffff`,
reflected input/output, and final XOR `0xffffffff`. The four checksum bytes are transmitted
big-endian.

Frame invariants:

- `START` is set exactly on offset zero. `END` is set exactly when
  `fragment_offset + payload_length == total_length`. A single-frame transfer sets both.
- Every frame in a transfer repeats the same version, transfer ID, total length, and checksum.
- Fragment offsets must be contiguous; overlap, repetition, or a gap resets the transfer.
- The client increments the transfer ID modulo `0xffff`, skipping zero. A response reuses the
  request transfer ID; logical correlation is still validated independently with `request_id`.
- Fragment payload capacity is `effective_characteristic_value_bytes - 12`. A connection unable to
  carry at least one payload byte is unusable.
- Each receiver keeps at most one in-progress transfer. A valid new `START` abandons any incomplete
  transfer before starting the new one.
- Reassembly resets after `3000 ms` without an accepted contiguous fragment, on a duplicate start,
  order violation, metadata mismatch, reserved flag, overflow, unsupported frame version, or
  checksum failure.
- Bytes are passed to the logical CBOR decoder only after exact-length reassembly and checksum
  verification. A failed transfer never invokes a protocol handler or timer behavior.

The frame is not part of the logical message model and is not stored in the journal or SQLite.
Future byte-stream transports carry the same CBOR envelope with their own framing.

## 10. Registry evolution checklist

Before committing a protocol registry change:

1. Assign new identifiers without editing or reusing existing assignments.
2. Classify the change as compatible minor or incompatible major.
3. Update explicit Rust and TypeScript codecs; do not enable generic serialization.
4. Add or retain canonical golden fixtures for every affected message and edge value.
5. Verify both languages encode byte-identical fixtures and decode all supported historical ones.
6. Verify malformed, duplicate-key, over-limit, and frame-order cases remain bounded and recoverable.
7. Audit the command list to confirm no remote timer control or sensitive provisioning was added.
