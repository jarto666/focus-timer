use core::str;

use heapless::{String, Vec};

use crate::model::{
    Capability, CatalogEntry, CatalogResult, ClockAnchorRequest, ClockAnchorResponse, DeviceEvent,
    ErrorCode, ErrorResponse, EventEnvelope, HelloResponse, JournalHealth, JournalStatus,
    MAX_CAPABILITIES, MAX_CUSTOM_PRESETS, MAX_FIRMWARE_VERSION_BYTES, MAX_LOGICAL_MESSAGE_BYTES,
    MAX_PRESET_ID_BYTES, MAX_PRESET_NAME_BYTES, MAX_PRODUCT_NAME_BYTES, MAX_RECORDS_PER_PAGE,
    MAX_SAFE_INTEGER, MAX_TOTAL_PRESETS, PresetCatalogResponse, PresetCatalogResultEvent,
    PresetSnapshot, ProposePresetCatalogRequest, ProposePresetCatalogResponse, ProtocolVersion,
    Request, RequestEnvelope, Response, ResponseEnvelope, SessionOutcome, SessionPageRequest,
    SessionPageResponse, SessionRecord, StatusResponse, ViewState,
};

const MAX_MAP_ENTRIES: usize = 16;
const MAX_ARRAY_ITEMS: usize = 8;
const MAX_TEXT_BYTES: usize = 32;
const MAX_NESTING_DEPTH: usize = 6;
const BUILT_IN_PRESET_IDS: [&str; 5] =
    ["deep-work", "focus", "pomodoro", "reading", "quick-sprint"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodeError {
    OutputTooSmall,
    MessageTooLarge,
    InvalidValue(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    Empty,
    MessageTooLarge,
    Truncated,
    NonCanonical,
    UnsupportedType,
    UnexpectedType,
    LimitExceeded,
    DuplicateOrUnorderedKey,
    InvalidUtf8,
    MissingField(u64),
    InvalidValue(u64),
    UnsupportedMessage(u64),
    TrailingData,
}

/// Encodes one request envelope into `output` using the constrained CBOR profile.
///
/// # Errors
///
/// Returns [`EncodeError`] when the model violates a registry invariant or the
/// output cannot hold the bounded message.
pub fn encode_request(envelope: &RequestEnvelope, output: &mut [u8]) -> Result<usize, EncodeError> {
    validate_request(envelope)?;
    let mut encoder = Encoder::new(output);
    encode_envelope_prefix(
        &mut encoder,
        envelope.version,
        envelope.request_id,
        envelope.request.message_kind(),
    )?;
    encode_request_payload(&mut encoder, &envelope.request)?;
    Ok(encoder.position)
}

/// Encodes one response envelope into `output` using the constrained CBOR profile.
///
/// # Errors
///
/// Returns [`EncodeError`] when the model violates a registry invariant or the
/// output cannot hold the bounded message.
pub fn encode_response(
    envelope: &ResponseEnvelope,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    validate_response(envelope)?;
    let mut encoder = Encoder::new(output);
    encode_envelope_prefix(
        &mut encoder,
        envelope.version,
        envelope.request_id,
        envelope.response.message_kind(),
    )?;
    encode_response_payload(&mut encoder, &envelope.response)?;
    Ok(encoder.position)
}

/// Encodes one unsolicited device event. Event envelopes always carry request ID zero.
/// Encodes one validated unsolicited event envelope.
///
/// # Errors
///
/// Returns a bounded encoding or model-validation error.
pub fn encode_event(envelope: &EventEnvelope, output: &mut [u8]) -> Result<usize, EncodeError> {
    validate_event(envelope)?;
    let mut encoder = Encoder::new(output);
    encode_envelope_prefix(
        &mut encoder,
        envelope.version,
        0,
        envelope.event.message_kind(),
    )?;
    encode_event_payload(&mut encoder, &envelope.event)?;
    Ok(encoder.position)
}

/// Decodes and validates one request envelope from constrained CBOR.
///
/// # Errors
///
/// Returns [`DecodeError`] for non-canonical, malformed, unsupported, trailing,
/// or over-limit input. No partially decoded request is returned.
pub fn decode_request(input: &[u8]) -> Result<RequestEnvelope, DecodeError> {
    let mut decoder = Decoder::new(input)?;
    let map_len = decoder.map_len()?;
    let mut previous_key = None;
    let mut major = None;
    let mut minor = None;
    let mut request_id = None;
    let mut message_kind = None;
    let mut request = None;

    for _ in 0..map_len {
        let key = decoder.map_key(&mut previous_key)?;
        match key {
            0 => major = Some(decoder.uint_u8(0)?),
            1 => minor = Some(decoder.uint_u8(1)?),
            2 => request_id = Some(decoder.uint_u32(2)?),
            3 => message_kind = Some(decoder.uint()?),
            4 => {
                let kind = message_kind.ok_or(DecodeError::MissingField(3))?;
                request = Some(decode_request_payload(
                    &mut decoder,
                    kind,
                    minor.ok_or(DecodeError::MissingField(1))?,
                )?);
            }
            _ => decoder.skip_value(1)?,
        }
    }

    decoder.finish()?;
    let envelope = RequestEnvelope {
        version: ProtocolVersion {
            major: major.ok_or(DecodeError::MissingField(0))?,
            minor: minor.ok_or(DecodeError::MissingField(1))?,
        },
        request_id: request_id.ok_or(DecodeError::MissingField(2))?,
        request: request.ok_or(DecodeError::MissingField(4))?,
    };
    validate_request(&envelope).map_err(|_| DecodeError::InvalidValue(4))?;
    Ok(envelope)
}

/// Decodes and validates one response envelope from constrained CBOR.
///
/// # Errors
///
/// Returns [`DecodeError`] for non-canonical, malformed, unsupported, trailing,
/// or over-limit input. No partially decoded response is returned.
pub fn decode_response(input: &[u8]) -> Result<ResponseEnvelope, DecodeError> {
    let mut decoder = Decoder::new(input)?;
    let map_len = decoder.map_len()?;
    let mut previous_key = None;
    let mut major = None;
    let mut minor = None;
    let mut request_id = None;
    let mut message_kind = None;
    let mut response = None;

    for _ in 0..map_len {
        let key = decoder.map_key(&mut previous_key)?;
        match key {
            0 => major = Some(decoder.uint_u8(0)?),
            1 => minor = Some(decoder.uint_u8(1)?),
            2 => request_id = Some(decoder.uint_u32(2)?),
            3 => message_kind = Some(decoder.uint()?),
            4 => {
                let kind = message_kind.ok_or(DecodeError::MissingField(3))?;
                response = Some(decode_response_payload(
                    &mut decoder,
                    kind,
                    minor.ok_or(DecodeError::MissingField(1))?,
                )?);
            }
            _ => decoder.skip_value(1)?,
        }
    }

    decoder.finish()?;
    let envelope = ResponseEnvelope {
        version: ProtocolVersion {
            major: major.ok_or(DecodeError::MissingField(0))?,
            minor: minor.ok_or(DecodeError::MissingField(1))?,
        },
        request_id: request_id.ok_or(DecodeError::MissingField(2))?,
        response: response.ok_or(DecodeError::MissingField(4))?,
    };
    validate_response(&envelope).map_err(|_| DecodeError::InvalidValue(4))?;
    Ok(envelope)
}

/// Decodes one unsolicited device event and requires the reserved zero request ID.
/// Decodes one complete constrained-CBOR unsolicited event envelope.
///
/// # Errors
///
/// Returns a canonical decoding or model-validation error.
pub fn decode_event(input: &[u8]) -> Result<EventEnvelope, DecodeError> {
    let mut decoder = Decoder::new(input)?;
    let map_len = decoder.map_len()?;
    let mut previous_key = None;
    let mut major = None;
    let mut minor = None;
    let mut request_id = None;
    let mut message_kind = None;
    let mut event = None;

    for _ in 0..map_len {
        let key = decoder.map_key(&mut previous_key)?;
        match key {
            0 => major = Some(decoder.uint_u8(0)?),
            1 => minor = Some(decoder.uint_u8(1)?),
            2 => request_id = Some(decoder.uint_u32(2)?),
            3 => message_kind = Some(decoder.uint()?),
            4 => {
                let kind = message_kind.ok_or(DecodeError::MissingField(3))?;
                event = Some(decode_event_payload(
                    &mut decoder,
                    kind,
                    minor.ok_or(DecodeError::MissingField(1))?,
                )?);
            }
            _ => decoder.skip_value(1)?,
        }
    }
    decoder.finish()?;
    if request_id.ok_or(DecodeError::MissingField(2))? != 0 {
        return Err(DecodeError::InvalidValue(2));
    }
    let envelope = EventEnvelope {
        version: ProtocolVersion {
            major: major.ok_or(DecodeError::MissingField(0))?,
            minor: minor.ok_or(DecodeError::MissingField(1))?,
        },
        event: event.ok_or(DecodeError::MissingField(4))?,
    };
    validate_event(&envelope).map_err(|_| DecodeError::InvalidValue(4))?;
    Ok(envelope)
}

fn encode_envelope_prefix(
    encoder: &mut Encoder<'_>,
    version: ProtocolVersion,
    request_id: u32,
    message_kind: u64,
) -> Result<(), EncodeError> {
    encoder.map(5)?;
    encoder.uint(0)?;
    encoder.uint(u64::from(version.major))?;
    encoder.uint(1)?;
    encoder.uint(u64::from(version.minor))?;
    encoder.uint(2)?;
    encoder.uint(u64::from(request_id))?;
    encoder.uint(3)?;
    encoder.uint(message_kind)?;
    encoder.uint(4)
}

fn encode_request_payload(encoder: &mut Encoder<'_>, request: &Request) -> Result<(), EncodeError> {
    match request {
        Request::Hello
        | Request::GetStatus
        | Request::GetPresetCatalog
        | Request::Unknown { .. } => encoder.map(0),
        Request::GetSessionPage(page) => {
            encoder.map(if page.journal_epoch.is_some() { 3 } else { 2 })?;
            if let Some(epoch) = page.journal_epoch {
                encoder.uint(0)?;
                encoder.bytes(&epoch)?;
            }
            encoder.uint(1)?;
            encoder.uint(page.after_sequence)?;
            encoder.uint(2)?;
            encoder.uint(u64::from(page.limit))
        }
        Request::SetClockAnchor(anchor) => {
            encoder.map(1)?;
            encoder.uint(0)?;
            encoder.uint(anchor.utc_ms)
        }
        Request::ProposePresetCatalog(proposal) => {
            encoder.map(3)?;
            encoder.uint(0)?;
            encoder.uint(proposal.expected_revision)?;
            encoder.uint(1)?;
            encoder.uint(u64::from(proposal.proposal_id))?;
            encoder.uint(2)?;
            encoder.array(proposal.custom_entries.len())?;
            for preset in &proposal.custom_entries {
                encode_preset(encoder, preset)?;
            }
            Ok(())
        }
    }
}

fn encode_response_payload(
    encoder: &mut Encoder<'_>,
    response: &Response,
) -> Result<(), EncodeError> {
    match response {
        Response::Hello(hello) => encode_hello(encoder, hello),
        Response::Status(status) => encode_status(encoder, status),
        Response::SessionPage(page) => encode_session_page(encoder, page),
        Response::ClockAnchor(anchor) => encode_clock_anchor(encoder, *anchor),
        Response::PresetCatalog(catalog) => encode_preset_catalog(encoder, catalog),
        Response::ProposePresetCatalog(proposal) => encode_catalog_proposal(encoder, *proposal),
        Response::Error(error) => encode_error(encoder, *error),
    }
}

fn encode_event_payload(encoder: &mut Encoder<'_>, event: &DeviceEvent) -> Result<(), EncodeError> {
    match event {
        DeviceEvent::LiveStatus(status) => encode_status(encoder, status),
        DeviceEvent::PresetCatalogResult(result) => encode_catalog_result(encoder, *result),
    }
}

fn encode_hello(encoder: &mut Encoder<'_>, hello: &HelloResponse) -> Result<(), EncodeError> {
    encoder.map(6)?;
    encoder.uint(0)?;
    encoder.bytes(&hello.device_id)?;
    encoder.uint(1)?;
    encoder.text(&hello.product_name)?;
    encoder.uint(2)?;
    encoder.text(&hello.firmware_version)?;
    encoder.uint(3)?;
    encoder.uint(u64::from(hello.supported_version.major))?;
    encoder.uint(4)?;
    encoder.uint(u64::from(hello.supported_version.minor))?;
    encoder.uint(5)?;
    encoder.array(hello.capabilities.len())?;
    for capability in &hello.capabilities {
        encoder.uint(u64::from(*capability as u8))?;
    }
    Ok(())
}

fn encode_status(encoder: &mut Encoder<'_>, status: &StatusResponse) -> Result<(), EncodeError> {
    let has_live_order = status.status_epoch.is_some();
    encoder.map(if has_live_order { 7 } else { 5 })?;
    encoder.uint(0)?;
    encoder.uint(u64::from(status.view_state as u8))?;
    encoder.uint(1)?;
    encode_preset(encoder, &status.preset)?;
    encoder.uint(2)?;
    encoder.uint(u64::from(status.remaining_duration_ms))?;
    encoder.uint(3)?;
    encode_journal_status(encoder, status.journal)?;
    encoder.uint(4)?;
    encoder.boolean(status.clock_known)?;
    if let Some(epoch) = status.status_epoch {
        encoder.uint(5)?;
        encoder.bytes(&epoch)?;
    }
    if let Some(revision) = status.status_revision {
        encoder.uint(6)?;
        encoder.uint(revision)?;
    }
    Ok(())
}

fn encode_preset(encoder: &mut Encoder<'_>, preset: &PresetSnapshot) -> Result<(), EncodeError> {
    encoder.map(3)?;
    encoder.uint(0)?;
    encoder.text(&preset.id)?;
    encoder.uint(1)?;
    encoder.text(&preset.name)?;
    encoder.uint(2)?;
    encoder.uint(u64::from(preset.planned_duration_ms))
}

fn encode_journal_status(
    encoder: &mut Encoder<'_>,
    journal: JournalStatus,
) -> Result<(), EncodeError> {
    let has_bounds = journal.oldest_sequence.is_some();
    encoder.map(if has_bounds { 4 } else { 2 })?;
    encoder.uint(0)?;
    encoder.bytes(&journal.epoch)?;
    if let Some(oldest) = journal.oldest_sequence {
        encoder.uint(1)?;
        encoder.uint(oldest)?;
    }
    if let Some(latest) = journal.latest_sequence {
        encoder.uint(2)?;
        encoder.uint(latest)?;
    }
    encoder.uint(3)?;
    encoder.uint(u64::from(journal.health as u8))
}

fn encode_session_page(
    encoder: &mut Encoder<'_>,
    page: &SessionPageResponse,
) -> Result<(), EncodeError> {
    let mut field_count = 4;
    if page.oldest_sequence.is_some() {
        field_count += 2;
    }
    if page.continuation_after_sequence.is_some() {
        field_count += 1;
    }
    encoder.map(field_count)?;
    encoder.uint(0)?;
    encoder.bytes(&page.journal_epoch)?;
    if let Some(oldest) = page.oldest_sequence {
        encoder.uint(1)?;
        encoder.uint(oldest)?;
    }
    if let Some(latest) = page.latest_sequence {
        encoder.uint(2)?;
        encoder.uint(latest)?;
    }
    encoder.uint(3)?;
    encoder.boolean(page.gap)?;
    encoder.uint(4)?;
    encoder.boolean(page.epoch_changed)?;
    encoder.uint(5)?;
    encoder.array(page.records.len())?;
    for record in &page.records {
        encode_session_record(encoder, record)?;
    }
    if let Some(continuation) = page.continuation_after_sequence {
        encoder.uint(6)?;
        encoder.uint(continuation)?;
    }
    Ok(())
}

fn encode_session_record(
    encoder: &mut Encoder<'_>,
    record: &SessionRecord,
) -> Result<(), EncodeError> {
    let mut field_count = 4;
    if record.started_at_utc_ms.is_some() {
        field_count += 1;
    }
    if record.ended_at_utc_ms.is_some() {
        field_count += 1;
    }
    encoder.map(field_count)?;
    encoder.uint(0)?;
    encoder.uint(record.sequence)?;
    encoder.uint(1)?;
    encode_preset(encoder, &record.preset)?;
    encoder.uint(2)?;
    encoder.uint(u64::from(record.active_duration_ms))?;
    encoder.uint(3)?;
    encoder.uint(u64::from(record.outcome as u8))?;
    if let Some(started_at) = record.started_at_utc_ms {
        encoder.uint(4)?;
        encoder.uint(started_at)?;
    }
    if let Some(ended_at) = record.ended_at_utc_ms {
        encoder.uint(5)?;
        encoder.uint(ended_at)?;
    }
    Ok(())
}

fn encode_clock_anchor(
    encoder: &mut Encoder<'_>,
    anchor: ClockAnchorResponse,
) -> Result<(), EncodeError> {
    encoder.map(2)?;
    encoder.uint(0)?;
    encoder.uint(anchor.accepted_utc_ms)?;
    encoder.uint(1)?;
    encoder.uint(anchor.device_monotonic_ms_at_receipt)
}

fn encode_preset_catalog(
    encoder: &mut Encoder<'_>,
    catalog: &PresetCatalogResponse,
) -> Result<(), EncodeError> {
    encoder.map(2)?;
    encoder.uint(0)?;
    encoder.uint(catalog.revision)?;
    encoder.uint(1)?;
    encoder.array(catalog.entries.len())?;
    for entry in &catalog.entries {
        encoder.map(4)?;
        encoder.uint(0)?;
        encoder.text(&entry.preset.id)?;
        encoder.uint(1)?;
        encoder.text(&entry.preset.name)?;
        encoder.uint(2)?;
        encoder.uint(u64::from(entry.preset.planned_duration_ms))?;
        encoder.uint(3)?;
        encoder.boolean(entry.built_in)?;
    }
    Ok(())
}

fn encode_catalog_proposal(
    encoder: &mut Encoder<'_>,
    proposal: ProposePresetCatalogResponse,
) -> Result<(), EncodeError> {
    encoder.map(2)?;
    encoder.uint(0)?;
    encoder.uint(u64::from(proposal.proposal_id))?;
    encoder.uint(1)?;
    encoder.uint(u64::from(proposal.expires_in_ms))
}

fn encode_catalog_result(
    encoder: &mut Encoder<'_>,
    result: PresetCatalogResultEvent,
) -> Result<(), EncodeError> {
    encoder.map(if result.catalog_revision.is_some() {
        3
    } else {
        2
    })?;
    encoder.uint(0)?;
    encoder.uint(u64::from(result.proposal_id))?;
    encoder.uint(1)?;
    encoder.uint(u64::from(result.result as u8))?;
    if let Some(revision) = result.catalog_revision {
        encoder.uint(2)?;
        encoder.uint(revision)?;
    }
    Ok(())
}

fn encode_error(encoder: &mut Encoder<'_>, error: ErrorResponse) -> Result<(), EncodeError> {
    let mut field_count = 1;
    if error.failed_message_kind.is_some() {
        field_count += 1;
    }
    if error.field_id.is_some() {
        field_count += 1;
    }
    if error.supported_version.is_some() {
        field_count += 2;
    }
    encoder.map(field_count)?;
    encoder.uint(0)?;
    encoder.uint(u64::from(error.code as u8))?;
    if let Some(kind) = error.failed_message_kind {
        encoder.uint(1)?;
        encoder.uint(kind)?;
    }
    if let Some(field_id) = error.field_id {
        encoder.uint(2)?;
        encoder.uint(field_id)?;
    }
    if let Some(version) = error.supported_version {
        encoder.uint(3)?;
        encoder.uint(u64::from(version.major))?;
        encoder.uint(4)?;
        encoder.uint(u64::from(version.minor))?;
    }
    Ok(())
}

fn decode_request_payload(
    decoder: &mut Decoder<'_>,
    kind: u64,
    protocol_minor: u8,
) -> Result<Request, DecodeError> {
    match kind {
        1 => {
            decoder.empty_map()?;
            Ok(Request::Hello)
        }
        3 => {
            decoder.empty_map()?;
            Ok(Request::GetStatus)
        }
        5 => decode_session_page_request(decoder).map(Request::GetSessionPage),
        7 => decode_clock_anchor_request(decoder).map(Request::SetClockAnchor),
        9 if protocol_minor >= 1 => {
            decoder.empty_map()?;
            Ok(Request::GetPresetCatalog)
        }
        11 if protocol_minor >= 1 => {
            decode_preset_catalog_proposal(decoder).map(Request::ProposePresetCatalog)
        }
        _ => {
            decoder.skip_map(2)?;
            Ok(Request::Unknown { message_kind: kind })
        }
    }
}

fn decode_response_payload(
    decoder: &mut Decoder<'_>,
    kind: u64,
    protocol_minor: u8,
) -> Result<Response, DecodeError> {
    match kind {
        2 => decode_hello(decoder).map(Response::Hello),
        4 => decode_status(decoder).map(Response::Status),
        6 => decode_session_page(decoder).map(Response::SessionPage),
        8 => decode_clock_anchor_response(decoder).map(Response::ClockAnchor),
        10 if protocol_minor >= 1 => decode_preset_catalog(decoder).map(Response::PresetCatalog),
        12 if protocol_minor >= 1 => {
            decode_catalog_proposal_response(decoder).map(Response::ProposePresetCatalog)
        }
        255 => decode_error(decoder).map(Response::Error),
        _ => Err(DecodeError::UnsupportedMessage(kind)),
    }
}

fn decode_event_payload(
    decoder: &mut Decoder<'_>,
    kind: u64,
    protocol_minor: u8,
) -> Result<DeviceEvent, DecodeError> {
    match kind {
        13 if protocol_minor >= 1 => decode_status(decoder).map(DeviceEvent::LiveStatus),
        14 if protocol_minor >= 1 => {
            decode_catalog_result(decoder).map(DeviceEvent::PresetCatalogResult)
        }
        _ => Err(DecodeError::UnsupportedMessage(kind)),
    }
}

fn decode_preset_catalog_proposal(
    decoder: &mut Decoder<'_>,
) -> Result<ProposePresetCatalogRequest, DecodeError> {
    let len = decoder.map_len()?;
    let mut previous = None;
    let mut expected_revision = None;
    let mut proposal_id = None;
    let mut custom_entries = None;
    for _ in 0..len {
        let key = decoder.map_key(&mut previous)?;
        match key {
            0 => expected_revision = Some(decoder.uint()?),
            1 => proposal_id = Some(decoder.uint_u32(1)?),
            2 => {
                let count = decoder.array_len()?;
                let mut entries = Vec::new();
                for _ in 0..count {
                    entries
                        .push(decode_preset(decoder)?)
                        .map_err(|_| DecodeError::LimitExceeded)?;
                }
                custom_entries = Some(entries);
            }
            _ => decoder.skip_value(2)?,
        }
    }
    Ok(ProposePresetCatalogRequest {
        expected_revision: expected_revision.ok_or(DecodeError::MissingField(0))?,
        proposal_id: proposal_id.ok_or(DecodeError::MissingField(1))?,
        custom_entries: custom_entries.ok_or(DecodeError::MissingField(2))?,
    })
}

fn decode_session_page_request(
    decoder: &mut Decoder<'_>,
) -> Result<SessionPageRequest, DecodeError> {
    let len = decoder.map_len()?;
    let mut previous = None;
    let mut epoch = None;
    let mut after = None;
    let mut limit = None;
    for _ in 0..len {
        let key = decoder.map_key(&mut previous)?;
        match key {
            0 => epoch = Some(decoder.bytes_exact::<8>(0)?),
            1 => after = Some(decoder.uint()?),
            2 => limit = Some(decoder.uint_u8(2)?),
            _ => decoder.skip_value(2)?,
        }
    }
    Ok(SessionPageRequest {
        journal_epoch: epoch,
        after_sequence: after.ok_or(DecodeError::MissingField(1))?,
        limit: limit.ok_or(DecodeError::MissingField(2))?,
    })
}

fn decode_clock_anchor_request(
    decoder: &mut Decoder<'_>,
) -> Result<ClockAnchorRequest, DecodeError> {
    let len = decoder.map_len()?;
    let mut previous = None;
    let mut utc_ms = None;
    for _ in 0..len {
        let key = decoder.map_key(&mut previous)?;
        match key {
            0 => utc_ms = Some(decoder.uint()?),
            _ => decoder.skip_value(2)?,
        }
    }
    Ok(ClockAnchorRequest {
        utc_ms: utc_ms.ok_or(DecodeError::MissingField(0))?,
    })
}

fn decode_hello(decoder: &mut Decoder<'_>) -> Result<HelloResponse, DecodeError> {
    let len = decoder.map_len()?;
    let mut previous = None;
    let mut device_id = None;
    let mut product_name = None;
    let mut firmware_version = None;
    let mut major = None;
    let mut minor = None;
    let mut capabilities = None;
    for _ in 0..len {
        let key = decoder.map_key(&mut previous)?;
        match key {
            0 => device_id = Some(decoder.bytes_exact::<16>(0)?),
            1 => product_name = Some(decoder.text::<MAX_PRODUCT_NAME_BYTES>(1)?),
            2 => firmware_version = Some(decoder.text::<MAX_FIRMWARE_VERSION_BYTES>(2)?),
            3 => major = Some(decoder.uint_u8(3)?),
            4 => minor = Some(decoder.uint_u8(4)?),
            5 => capabilities = Some(decode_capabilities(decoder)?),
            _ => decoder.skip_value(2)?,
        }
    }
    Ok(HelloResponse {
        device_id: device_id.ok_or(DecodeError::MissingField(0))?,
        product_name: product_name.ok_or(DecodeError::MissingField(1))?,
        firmware_version: firmware_version.ok_or(DecodeError::MissingField(2))?,
        supported_version: ProtocolVersion {
            major: major.ok_or(DecodeError::MissingField(3))?,
            minor: minor.ok_or(DecodeError::MissingField(4))?,
        },
        capabilities: capabilities.ok_or(DecodeError::MissingField(5))?,
    })
}

fn decode_capabilities(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<Capability, MAX_CAPABILITIES>, DecodeError> {
    let len = decoder.array_len()?;
    let mut result = Vec::new();
    let mut previous = None;
    for _ in 0..len {
        let raw = decoder.uint()?;
        if previous.is_some_and(|value| raw <= value) {
            return Err(DecodeError::InvalidValue(5));
        }
        previous = Some(raw);
        if let Some(capability) = Capability::from_wire(raw) {
            result
                .push(capability)
                .map_err(|_| DecodeError::LimitExceeded)?;
        }
    }
    Ok(result)
}

fn decode_status(decoder: &mut Decoder<'_>) -> Result<StatusResponse, DecodeError> {
    let len = decoder.map_len()?;
    let mut previous = None;
    let mut view_state = None;
    let mut preset = None;
    let mut remaining = None;
    let mut journal = None;
    let mut clock_known = None;
    let mut status_epoch = None;
    let mut status_revision = None;
    for _ in 0..len {
        let key = decoder.map_key(&mut previous)?;
        match key {
            0 => {
                view_state = Some(
                    ViewState::from_wire(decoder.uint()?).ok_or(DecodeError::InvalidValue(0))?,
                );
            }
            1 => preset = Some(decode_preset(decoder)?),
            2 => remaining = Some(decoder.uint_u32(2)?),
            3 => journal = Some(decode_journal_status(decoder)?),
            4 => clock_known = Some(decoder.boolean()?),
            5 => status_epoch = Some(decoder.bytes_exact::<8>(5)?),
            6 => status_revision = Some(decoder.uint()?),
            _ => decoder.skip_value(2)?,
        }
    }
    Ok(StatusResponse {
        view_state: view_state.ok_or(DecodeError::MissingField(0))?,
        preset: preset.ok_or(DecodeError::MissingField(1))?,
        remaining_duration_ms: remaining.ok_or(DecodeError::MissingField(2))?,
        journal: journal.ok_or(DecodeError::MissingField(3))?,
        clock_known: clock_known.ok_or(DecodeError::MissingField(4))?,
        status_epoch,
        status_revision,
    })
}

fn decode_preset_catalog(decoder: &mut Decoder<'_>) -> Result<PresetCatalogResponse, DecodeError> {
    let len = decoder.map_len()?;
    let mut previous = None;
    let mut revision = None;
    let mut entries = None;
    for _ in 0..len {
        let key = decoder.map_key(&mut previous)?;
        match key {
            0 => revision = Some(decoder.uint()?),
            1 => {
                let count = decoder.array_len_bounded(MAX_TOTAL_PRESETS)?;
                let mut entries_value = Vec::new();
                for _ in 0..count {
                    entries_value
                        .push(decode_catalog_entry(decoder)?)
                        .map_err(|_| DecodeError::LimitExceeded)?;
                }
                entries = Some(entries_value);
            }
            _ => decoder.skip_value(2)?,
        }
    }
    Ok(PresetCatalogResponse {
        revision: revision.ok_or(DecodeError::MissingField(0))?,
        entries: entries.ok_or(DecodeError::MissingField(1))?,
    })
}

fn decode_catalog_entry(decoder: &mut Decoder<'_>) -> Result<CatalogEntry, DecodeError> {
    let len = decoder.map_len()?;
    let mut previous = None;
    let mut id = None;
    let mut name = None;
    let mut duration = None;
    let mut built_in = None;
    for _ in 0..len {
        let key = decoder.map_key(&mut previous)?;
        match key {
            0 => id = Some(decoder.text::<MAX_PRESET_ID_BYTES>(0)?),
            1 => name = Some(decoder.text::<MAX_PRESET_NAME_BYTES>(1)?),
            2 => duration = Some(decoder.uint_u32(2)?),
            3 => built_in = Some(decoder.boolean()?),
            _ => decoder.skip_value(3)?,
        }
    }
    Ok(CatalogEntry {
        preset: PresetSnapshot {
            id: id.ok_or(DecodeError::MissingField(0))?,
            name: name.ok_or(DecodeError::MissingField(1))?,
            planned_duration_ms: duration.ok_or(DecodeError::MissingField(2))?,
        },
        built_in: built_in.ok_or(DecodeError::MissingField(3))?,
    })
}

fn decode_catalog_proposal_response(
    decoder: &mut Decoder<'_>,
) -> Result<ProposePresetCatalogResponse, DecodeError> {
    let len = decoder.map_len()?;
    let mut previous = None;
    let mut proposal_id = None;
    let mut expires_in_ms = None;
    for _ in 0..len {
        let key = decoder.map_key(&mut previous)?;
        match key {
            0 => proposal_id = Some(decoder.uint_u32(0)?),
            1 => expires_in_ms = Some(decoder.uint_u32(1)?),
            _ => decoder.skip_value(2)?,
        }
    }
    Ok(ProposePresetCatalogResponse {
        proposal_id: proposal_id.ok_or(DecodeError::MissingField(0))?,
        expires_in_ms: expires_in_ms.ok_or(DecodeError::MissingField(1))?,
    })
}

fn decode_catalog_result(
    decoder: &mut Decoder<'_>,
) -> Result<PresetCatalogResultEvent, DecodeError> {
    let len = decoder.map_len()?;
    let mut previous = None;
    let mut proposal_id = None;
    let mut result = None;
    let mut catalog_revision = None;
    for _ in 0..len {
        let key = decoder.map_key(&mut previous)?;
        match key {
            0 => proposal_id = Some(decoder.uint_u32(0)?),
            1 => {
                result = Some(
                    CatalogResult::from_wire(decoder.uint()?)
                        .ok_or(DecodeError::InvalidValue(1))?,
                );
            }
            2 => catalog_revision = Some(decoder.uint()?),
            _ => decoder.skip_value(2)?,
        }
    }
    Ok(PresetCatalogResultEvent {
        proposal_id: proposal_id.ok_or(DecodeError::MissingField(0))?,
        result: result.ok_or(DecodeError::MissingField(1))?,
        catalog_revision,
    })
}

fn decode_preset(decoder: &mut Decoder<'_>) -> Result<PresetSnapshot, DecodeError> {
    let len = decoder.map_len()?;
    let mut previous = None;
    let mut id = None;
    let mut name = None;
    let mut duration = None;
    for _ in 0..len {
        let key = decoder.map_key(&mut previous)?;
        match key {
            0 => id = Some(decoder.text::<MAX_PRESET_ID_BYTES>(0)?),
            1 => name = Some(decoder.text::<MAX_PRESET_NAME_BYTES>(1)?),
            2 => duration = Some(decoder.uint_u32(2)?),
            _ => decoder.skip_value(3)?,
        }
    }
    Ok(PresetSnapshot {
        id: id.ok_or(DecodeError::MissingField(0))?,
        name: name.ok_or(DecodeError::MissingField(1))?,
        planned_duration_ms: duration.ok_or(DecodeError::MissingField(2))?,
    })
}

fn decode_journal_status(decoder: &mut Decoder<'_>) -> Result<JournalStatus, DecodeError> {
    let len = decoder.map_len()?;
    let mut previous = None;
    let mut epoch = None;
    let mut oldest = None;
    let mut latest = None;
    let mut health = None;
    for _ in 0..len {
        let key = decoder.map_key(&mut previous)?;
        match key {
            0 => epoch = Some(decoder.bytes_exact::<8>(0)?),
            1 => oldest = Some(decoder.uint()?),
            2 => latest = Some(decoder.uint()?),
            3 => {
                health = Some(
                    JournalHealth::from_wire(decoder.uint()?)
                        .ok_or(DecodeError::InvalidValue(3))?,
                );
            }
            _ => decoder.skip_value(3)?,
        }
    }
    Ok(JournalStatus {
        epoch: epoch.ok_or(DecodeError::MissingField(0))?,
        oldest_sequence: oldest,
        latest_sequence: latest,
        health: health.ok_or(DecodeError::MissingField(3))?,
    })
}

fn decode_session_page(decoder: &mut Decoder<'_>) -> Result<SessionPageResponse, DecodeError> {
    let len = decoder.map_len()?;
    let mut previous = None;
    let mut epoch = None;
    let mut oldest = None;
    let mut latest = None;
    let mut gap = None;
    let mut epoch_changed = None;
    let mut records = None;
    let mut continuation = None;
    for _ in 0..len {
        let key = decoder.map_key(&mut previous)?;
        match key {
            0 => epoch = Some(decoder.bytes_exact::<8>(0)?),
            1 => oldest = Some(decoder.uint()?),
            2 => latest = Some(decoder.uint()?),
            3 => gap = Some(decoder.boolean()?),
            4 => epoch_changed = Some(decoder.boolean()?),
            5 => records = Some(decode_session_records(decoder)?),
            6 => continuation = Some(decoder.uint()?),
            _ => decoder.skip_value(2)?,
        }
    }
    Ok(SessionPageResponse {
        journal_epoch: epoch.ok_or(DecodeError::MissingField(0))?,
        oldest_sequence: oldest,
        latest_sequence: latest,
        gap: gap.ok_or(DecodeError::MissingField(3))?,
        epoch_changed: epoch_changed.ok_or(DecodeError::MissingField(4))?,
        records: records.ok_or(DecodeError::MissingField(5))?,
        continuation_after_sequence: continuation,
    })
}

fn decode_session_records(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<SessionRecord, MAX_RECORDS_PER_PAGE>, DecodeError> {
    let len = decoder.array_len()?;
    let mut records = Vec::new();
    for _ in 0..len {
        records
            .push(decode_session_record(decoder)?)
            .map_err(|_| DecodeError::LimitExceeded)?;
    }
    Ok(records)
}

fn decode_session_record(decoder: &mut Decoder<'_>) -> Result<SessionRecord, DecodeError> {
    let len = decoder.map_len()?;
    let mut previous = None;
    let mut sequence = None;
    let mut preset = None;
    let mut active_duration = None;
    let mut outcome = None;
    let mut started_at = None;
    let mut ended_at = None;
    for _ in 0..len {
        let key = decoder.map_key(&mut previous)?;
        match key {
            0 => sequence = Some(decoder.uint()?),
            1 => preset = Some(decode_preset(decoder)?),
            2 => active_duration = Some(decoder.uint_u32(2)?),
            3 => {
                outcome = Some(
                    SessionOutcome::from_wire(decoder.uint()?)
                        .ok_or(DecodeError::InvalidValue(3))?,
                );
            }
            4 => started_at = Some(decoder.uint()?),
            5 => ended_at = Some(decoder.uint()?),
            _ => decoder.skip_value(4)?,
        }
    }
    Ok(SessionRecord {
        sequence: sequence.ok_or(DecodeError::MissingField(0))?,
        preset: preset.ok_or(DecodeError::MissingField(1))?,
        active_duration_ms: active_duration.ok_or(DecodeError::MissingField(2))?,
        outcome: outcome.ok_or(DecodeError::MissingField(3))?,
        started_at_utc_ms: started_at,
        ended_at_utc_ms: ended_at,
    })
}

fn decode_clock_anchor_response(
    decoder: &mut Decoder<'_>,
) -> Result<ClockAnchorResponse, DecodeError> {
    let len = decoder.map_len()?;
    let mut previous = None;
    let mut utc_ms = None;
    let mut monotonic_ms = None;
    for _ in 0..len {
        let key = decoder.map_key(&mut previous)?;
        match key {
            0 => utc_ms = Some(decoder.uint()?),
            1 => monotonic_ms = Some(decoder.uint()?),
            _ => decoder.skip_value(2)?,
        }
    }
    Ok(ClockAnchorResponse {
        accepted_utc_ms: utc_ms.ok_or(DecodeError::MissingField(0))?,
        device_monotonic_ms_at_receipt: monotonic_ms.ok_or(DecodeError::MissingField(1))?,
    })
}

fn decode_error(decoder: &mut Decoder<'_>) -> Result<ErrorResponse, DecodeError> {
    let len = decoder.map_len()?;
    let mut previous = None;
    let mut code = None;
    let mut failed_kind = None;
    let mut field_id = None;
    let mut major = None;
    let mut minor = None;
    for _ in 0..len {
        let key = decoder.map_key(&mut previous)?;
        match key {
            0 => {
                code = Some(
                    ErrorCode::from_wire(decoder.uint()?).ok_or(DecodeError::InvalidValue(0))?,
                );
            }
            1 => failed_kind = Some(decoder.uint()?),
            2 => field_id = Some(decoder.uint()?),
            3 => major = Some(decoder.uint_u8(3)?),
            4 => minor = Some(decoder.uint_u8(4)?),
            _ => decoder.skip_value(2)?,
        }
    }
    let supported_version = match (major, minor) {
        (Some(major), Some(minor)) => Some(ProtocolVersion { major, minor }),
        (None, None) => None,
        _ => return Err(DecodeError::InvalidValue(3)),
    };
    Ok(ErrorResponse {
        code: code.ok_or(DecodeError::MissingField(0))?,
        failed_message_kind: failed_kind,
        field_id,
        supported_version,
    })
}

fn validate_request(envelope: &RequestEnvelope) -> Result<(), EncodeError> {
    validate_request_id(envelope.request_id)?;
    match &envelope.request {
        Request::GetSessionPage(page) => {
            validate_safe(page.after_sequence, "after_sequence")?;
            if page.limit == 0 || usize::from(page.limit) > MAX_RECORDS_PER_PAGE {
                return Err(EncodeError::InvalidValue("limit"));
            }
            if page.journal_epoch.is_none() && page.after_sequence != 0 {
                return Err(EncodeError::InvalidValue("journal_epoch"));
            }
        }
        Request::SetClockAnchor(anchor) => validate_safe(anchor.utc_ms, "utc_ms")?,
        Request::ProposePresetCatalog(proposal) => validate_catalog_proposal(proposal)?,
        Request::Unknown { message_kind } if *message_kind == 0 => {
            return Err(EncodeError::InvalidValue("message_kind"));
        }
        Request::Hello
        | Request::GetStatus
        | Request::GetPresetCatalog
        | Request::Unknown { .. } => {}
    }
    Ok(())
}

fn validate_response(envelope: &ResponseEnvelope) -> Result<(), EncodeError> {
    validate_request_id(envelope.request_id)?;
    match &envelope.response {
        Response::Hello(hello) => validate_hello(hello),
        Response::Status(status) => validate_status(status),
        Response::SessionPage(page) => validate_page(page),
        Response::ClockAnchor(anchor) => {
            validate_safe(anchor.accepted_utc_ms, "accepted_utc_ms")?;
            validate_safe(
                anchor.device_monotonic_ms_at_receipt,
                "device_monotonic_ms_at_receipt",
            )
        }
        Response::PresetCatalog(catalog) => validate_catalog(catalog),
        Response::ProposePresetCatalog(proposal) => {
            validate_nonzero_id(proposal.proposal_id, "proposal_id")?;
            if proposal.expires_in_ms == 0 {
                return Err(EncodeError::InvalidValue("expires_in_ms"));
            }
            Ok(())
        }
        Response::Error(error) => validate_error(*error),
    }
}

fn validate_event(envelope: &EventEnvelope) -> Result<(), EncodeError> {
    match &envelope.event {
        DeviceEvent::LiveStatus(status) => {
            validate_status(status)?;
            if status.status_epoch.is_none() {
                return Err(EncodeError::InvalidValue("status_epoch"));
            }
            Ok(())
        }
        DeviceEvent::PresetCatalogResult(result) => validate_catalog_result(*result),
    }
}

fn validate_request_id(request_id: u32) -> Result<(), EncodeError> {
    if request_id == 0 {
        return Err(EncodeError::InvalidValue("request_id"));
    }
    Ok(())
}

fn validate_hello(hello: &HelloResponse) -> Result<(), EncodeError> {
    validate_text(&hello.product_name, "product_name")?;
    validate_text(&hello.firmware_version, "firmware_version")?;
    let mut previous = None;
    for capability in &hello.capabilities {
        let raw = *capability as u8;
        if previous.is_some_and(|value| raw <= value) {
            return Err(EncodeError::InvalidValue("capabilities"));
        }
        previous = Some(raw);
    }
    Ok(())
}

fn validate_status(status: &StatusResponse) -> Result<(), EncodeError> {
    validate_preset(&status.preset)?;
    validate_bounds(
        status.journal.oldest_sequence,
        status.journal.latest_sequence,
    )?;
    match (status.status_epoch, status.status_revision) {
        (Some(_), Some(revision)) if revision != 0 => validate_safe(revision, "status_revision"),
        (None, None) => Ok(()),
        _ => Err(EncodeError::InvalidValue("status_epoch")),
    }
}

fn validate_catalog(catalog: &PresetCatalogResponse) -> Result<(), EncodeError> {
    validate_safe(catalog.revision, "catalog_revision")?;
    if catalog.entries.len() < 5 || catalog.entries.len() > MAX_TOTAL_PRESETS {
        return Err(EncodeError::InvalidValue("catalog_entries"));
    }
    let mut custom_seen = false;
    let mut built_in_count = 0;
    for (index, entry) in catalog.entries.iter().enumerate() {
        validate_preset(&entry.preset)?;
        if entry.built_in {
            if custom_seen {
                return Err(EncodeError::InvalidValue("built_in"));
            }
            built_in_count += 1;
        } else {
            custom_seen = true;
            validate_custom_preset(&entry.preset)?;
        }
        if catalog.entries[..index]
            .iter()
            .any(|candidate| candidate.preset.id == entry.preset.id)
        {
            return Err(EncodeError::InvalidValue("preset_id"));
        }
    }
    if built_in_count != 5 || catalog.entries.len() - built_in_count > MAX_CUSTOM_PRESETS {
        return Err(EncodeError::InvalidValue("built_in"));
    }
    Ok(())
}

fn validate_catalog_proposal(proposal: &ProposePresetCatalogRequest) -> Result<(), EncodeError> {
    validate_safe(proposal.expected_revision, "expected_revision")?;
    validate_nonzero_id(proposal.proposal_id, "proposal_id")?;
    for (index, preset) in proposal.custom_entries.iter().enumerate() {
        validate_custom_preset(preset)?;
        if BUILT_IN_PRESET_IDS.contains(&preset.id.as_str()) {
            return Err(EncodeError::InvalidValue("preset_id"));
        }
        if proposal.custom_entries[..index]
            .iter()
            .any(|candidate| candidate.id == preset.id)
        {
            return Err(EncodeError::InvalidValue("preset_id"));
        }
    }
    Ok(())
}

fn validate_custom_preset(preset: &PresetSnapshot) -> Result<(), EncodeError> {
    validate_preset(preset)?;
    if preset.planned_duration_ms < 60_000
        || preset.planned_duration_ms > 43_200_000
        || preset.planned_duration_ms % 60_000 != 0
    {
        return Err(EncodeError::InvalidValue("planned_duration_ms"));
    }
    Ok(())
}

fn validate_catalog_result(result: PresetCatalogResultEvent) -> Result<(), EncodeError> {
    validate_nonzero_id(result.proposal_id, "proposal_id")?;
    match (result.result, result.catalog_revision) {
        (CatalogResult::Committed, Some(revision)) if revision != 0 => {
            validate_safe(revision, "catalog_revision")
        }
        (_, None) => Ok(()),
        (CatalogResult::Committed, _) | (_, Some(_)) => {
            Err(EncodeError::InvalidValue("catalog_revision"))
        }
    }
}

fn validate_nonzero_id(value: u32, field: &'static str) -> Result<(), EncodeError> {
    if value == 0 {
        return Err(EncodeError::InvalidValue(field));
    }
    Ok(())
}

fn validate_preset(preset: &PresetSnapshot) -> Result<(), EncodeError> {
    validate_text(&preset.id, "preset_id")?;
    validate_text(&preset.name, "preset_name")?;
    if preset.planned_duration_ms == 0 {
        return Err(EncodeError::InvalidValue("planned_duration_ms"));
    }
    Ok(())
}

fn validate_page(page: &SessionPageResponse) -> Result<(), EncodeError> {
    validate_bounds(page.oldest_sequence, page.latest_sequence)?;
    if page.gap && page.epoch_changed {
        return Err(EncodeError::InvalidValue("gap"));
    }
    if page.oldest_sequence.is_none()
        && (!page.records.is_empty() || page.continuation_after_sequence.is_some())
    {
        return Err(EncodeError::InvalidValue("records"));
    }

    let mut previous = None;
    for record in &page.records {
        validate_record(record)?;
        if previous.is_some_and(|value| record.sequence <= value) {
            return Err(EncodeError::InvalidValue("sequence"));
        }
        if page
            .oldest_sequence
            .is_some_and(|oldest| record.sequence < oldest)
            || page
                .latest_sequence
                .is_some_and(|latest| record.sequence > latest)
        {
            return Err(EncodeError::InvalidValue("sequence"));
        }
        previous = Some(record.sequence);
    }
    if let Some(continuation) = page.continuation_after_sequence {
        if previous != Some(continuation)
            || page
                .latest_sequence
                .is_none_or(|latest| continuation >= latest)
        {
            return Err(EncodeError::InvalidValue("continuation_after_sequence"));
        }
    }
    Ok(())
}

fn validate_record(record: &SessionRecord) -> Result<(), EncodeError> {
    validate_safe(record.sequence, "sequence")?;
    if record.sequence == 0 {
        return Err(EncodeError::InvalidValue("sequence"));
    }
    validate_preset(&record.preset)?;
    if record.active_duration_ms > record.preset.planned_duration_ms {
        return Err(EncodeError::InvalidValue("active_duration_ms"));
    }
    if let Some(started_at) = record.started_at_utc_ms {
        validate_safe(started_at, "started_at_utc_ms")?;
    }
    if let Some(ended_at) = record.ended_at_utc_ms {
        validate_safe(ended_at, "ended_at_utc_ms")?;
    }
    if let (Some(started_at), Some(ended_at)) = (record.started_at_utc_ms, record.ended_at_utc_ms)
        && started_at > ended_at
    {
        return Err(EncodeError::InvalidValue("ended_at_utc_ms"));
    }
    Ok(())
}

fn validate_bounds(oldest: Option<u64>, latest: Option<u64>) -> Result<(), EncodeError> {
    match (oldest, latest) {
        (Some(oldest), Some(latest)) if oldest != 0 && oldest <= latest => {
            validate_safe(oldest, "oldest_sequence")?;
            validate_safe(latest, "latest_sequence")
        }
        (None, None) => Ok(()),
        _ => Err(EncodeError::InvalidValue("journal_bounds")),
    }
}

fn validate_error(error: ErrorResponse) -> Result<(), EncodeError> {
    if error.code == ErrorCode::UnsupportedProtocolVersion && error.supported_version.is_none() {
        return Err(EncodeError::InvalidValue("supported_version"));
    }
    if error.code != ErrorCode::UnsupportedProtocolVersion && error.supported_version.is_some() {
        return Err(EncodeError::InvalidValue("supported_version"));
    }
    Ok(())
}

fn validate_text<const N: usize>(
    value: &String<N>,
    field: &'static str,
) -> Result<(), EncodeError> {
    if value.is_empty() {
        return Err(EncodeError::InvalidValue(field));
    }
    Ok(())
}

fn validate_safe(value: u64, field: &'static str) -> Result<(), EncodeError> {
    if value > MAX_SAFE_INTEGER {
        return Err(EncodeError::InvalidValue(field));
    }
    Ok(())
}

struct Encoder<'a> {
    output: &'a mut [u8],
    position: usize,
    overflow_error: EncodeError,
}

impl<'a> Encoder<'a> {
    fn new(output: &'a mut [u8]) -> Self {
        let overflow_error = if output.len() < MAX_LOGICAL_MESSAGE_BYTES {
            EncodeError::OutputTooSmall
        } else {
            EncodeError::MessageTooLarge
        };
        let limit = output.len().min(MAX_LOGICAL_MESSAGE_BYTES);
        Self {
            output: &mut output[..limit],
            position: 0,
            overflow_error,
        }
    }

    fn uint(&mut self, value: u64) -> Result<(), EncodeError> {
        self.head(0, value)
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), EncodeError> {
        self.head(
            2,
            u64::try_from(value.len()).map_err(|_| EncodeError::MessageTooLarge)?,
        )?;
        self.put_slice(value)
    }

    fn text(&mut self, value: &str) -> Result<(), EncodeError> {
        self.head(
            3,
            u64::try_from(value.len()).map_err(|_| EncodeError::MessageTooLarge)?,
        )?;
        self.put_slice(value.as_bytes())
    }

    fn array(&mut self, len: usize) -> Result<(), EncodeError> {
        self.head(
            4,
            u64::try_from(len).map_err(|_| EncodeError::MessageTooLarge)?,
        )
    }

    fn map(&mut self, len: usize) -> Result<(), EncodeError> {
        self.head(
            5,
            u64::try_from(len).map_err(|_| EncodeError::MessageTooLarge)?,
        )
    }

    fn boolean(&mut self, value: bool) -> Result<(), EncodeError> {
        self.put(if value { 0xf5 } else { 0xf4 })
    }

    fn head(&mut self, major: u8, value: u64) -> Result<(), EncodeError> {
        let prefix = major << 5;
        match value {
            0..=23 => self.put(prefix | u8::try_from(value).expect("value is at most 23")),
            24..=0xff => {
                self.put(prefix | 0x18)?;
                self.put(u8::try_from(value).expect("value is at most u8::MAX"))
            }
            0x100..=0xffff => {
                self.put(prefix | 0x19)?;
                self.put_slice(
                    &u16::try_from(value)
                        .expect("value is at most u16::MAX")
                        .to_be_bytes(),
                )
            }
            0x1_0000..=0xffff_ffff => {
                self.put(prefix | 0x1a)?;
                self.put_slice(
                    &u32::try_from(value)
                        .expect("value is at most u32::MAX")
                        .to_be_bytes(),
                )
            }
            _ => {
                self.put(prefix | 0x1b)?;
                self.put_slice(&value.to_be_bytes())
            }
        }
    }

    fn put(&mut self, value: u8) -> Result<(), EncodeError> {
        let slot = self
            .output
            .get_mut(self.position)
            .ok_or(self.overflow_error)?;
        *slot = value;
        self.position += 1;
        Ok(())
    }

    fn put_slice(&mut self, value: &[u8]) -> Result<(), EncodeError> {
        let end = self
            .position
            .checked_add(value.len())
            .ok_or(self.overflow_error)?;
        let target = self
            .output
            .get_mut(self.position..end)
            .ok_or(self.overflow_error)?;
        target.copy_from_slice(value);
        self.position = end;
        Ok(())
    }
}

struct Decoder<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> Decoder<'a> {
    fn new(input: &'a [u8]) -> Result<Self, DecodeError> {
        if input.is_empty() {
            return Err(DecodeError::Empty);
        }
        if input.len() > MAX_LOGICAL_MESSAGE_BYTES {
            return Err(DecodeError::MessageTooLarge);
        }
        Ok(Self { input, position: 0 })
    }

    fn finish(&self) -> Result<(), DecodeError> {
        if self.position == self.input.len() {
            Ok(())
        } else {
            Err(DecodeError::TrailingData)
        }
    }

    fn uint(&mut self) -> Result<u64, DecodeError> {
        let (major, value) = self.head()?;
        if major != 0 {
            return Err(DecodeError::UnexpectedType);
        }
        Ok(value)
    }

    fn uint_u8(&mut self, field: u64) -> Result<u8, DecodeError> {
        u8::try_from(self.uint()?).map_err(|_| DecodeError::InvalidValue(field))
    }

    fn uint_u32(&mut self, field: u64) -> Result<u32, DecodeError> {
        u32::try_from(self.uint()?).map_err(|_| DecodeError::InvalidValue(field))
    }

    fn map_len(&mut self) -> Result<usize, DecodeError> {
        let (major, value) = self.head()?;
        if major != 5 {
            return Err(DecodeError::UnexpectedType);
        }
        let len = usize::try_from(value).map_err(|_| DecodeError::LimitExceeded)?;
        if len > MAX_MAP_ENTRIES {
            return Err(DecodeError::LimitExceeded);
        }
        Ok(len)
    }

    fn array_len(&mut self) -> Result<usize, DecodeError> {
        self.array_len_bounded(MAX_ARRAY_ITEMS)
    }

    fn array_len_bounded(&mut self, maximum: usize) -> Result<usize, DecodeError> {
        let (major, value) = self.head()?;
        if major != 4 {
            return Err(DecodeError::UnexpectedType);
        }
        let len = usize::try_from(value).map_err(|_| DecodeError::LimitExceeded)?;
        if len > maximum {
            return Err(DecodeError::LimitExceeded);
        }
        Ok(len)
    }

    fn empty_map(&mut self) -> Result<(), DecodeError> {
        if self.map_len()? == 0 {
            Ok(())
        } else {
            Err(DecodeError::InvalidValue(4))
        }
    }

    fn map_key(&mut self, previous: &mut Option<u64>) -> Result<u64, DecodeError> {
        let key = self.uint()?;
        if previous.is_some_and(|value| key <= value) {
            return Err(DecodeError::DuplicateOrUnorderedKey);
        }
        *previous = Some(key);
        Ok(key)
    }

    fn bytes_exact<const N: usize>(&mut self, field: u64) -> Result<[u8; N], DecodeError> {
        let (major, value) = self.head()?;
        if major != 2 {
            return Err(DecodeError::UnexpectedType);
        }
        let len = usize::try_from(value).map_err(|_| DecodeError::LimitExceeded)?;
        if len != N {
            return Err(DecodeError::InvalidValue(field));
        }
        let source = self.take(len)?;
        source
            .try_into()
            .map_err(|_| DecodeError::InvalidValue(field))
    }

    fn text<const N: usize>(&mut self, field: u64) -> Result<String<N>, DecodeError> {
        let (major, value) = self.head()?;
        if major != 3 {
            return Err(DecodeError::UnexpectedType);
        }
        let len = usize::try_from(value).map_err(|_| DecodeError::LimitExceeded)?;
        if len > N || len > MAX_TEXT_BYTES {
            return Err(DecodeError::LimitExceeded);
        }
        let value = str::from_utf8(self.take(len)?).map_err(|_| DecodeError::InvalidUtf8)?;
        let mut result = String::new();
        result
            .push_str(value)
            .map_err(|()| DecodeError::InvalidValue(field))?;
        Ok(result)
    }

    fn boolean(&mut self) -> Result<bool, DecodeError> {
        match self.byte()? {
            0xf4 => Ok(false),
            0xf5 => Ok(true),
            _ => Err(DecodeError::UnexpectedType),
        }
    }

    fn skip_map(&mut self, depth: usize) -> Result<(), DecodeError> {
        let len = self.map_len()?;
        let mut previous = None;
        for _ in 0..len {
            self.map_key(&mut previous)?;
            self.skip_value(depth + 1)?;
        }
        Ok(())
    }

    fn skip_value(&mut self, depth: usize) -> Result<(), DecodeError> {
        if depth > MAX_NESTING_DEPTH {
            return Err(DecodeError::LimitExceeded);
        }
        let initial = *self
            .input
            .get(self.position)
            .ok_or(DecodeError::Truncated)?;
        let major = initial >> 5;
        match major {
            0 => {
                self.head()?;
                Ok(())
            }
            2 | 3 => {
                let (_, raw_len) = self.head()?;
                let len = usize::try_from(raw_len).map_err(|_| DecodeError::LimitExceeded)?;
                if len > MAX_TEXT_BYTES {
                    return Err(DecodeError::LimitExceeded);
                }
                let bytes = self.take(len)?;
                if major == 3 {
                    str::from_utf8(bytes).map_err(|_| DecodeError::InvalidUtf8)?;
                }
                Ok(())
            }
            4 => {
                let len = self.array_len()?;
                for _ in 0..len {
                    self.skip_value(depth + 1)?;
                }
                Ok(())
            }
            5 => self.skip_map(depth),
            7 => match self.byte()? {
                0xf4 | 0xf5 => Ok(()),
                _ => Err(DecodeError::UnsupportedType),
            },
            _ => Err(DecodeError::UnsupportedType),
        }
    }

    fn head(&mut self) -> Result<(u8, u64), DecodeError> {
        let initial = self.byte()?;
        let major = initial >> 5;
        let additional = initial & 0x1f;
        let value = match additional {
            0..=23 => u64::from(additional),
            24 => {
                let value = u64::from(self.byte()?);
                if value < 24 {
                    return Err(DecodeError::NonCanonical);
                }
                value
            }
            25 => {
                let value = u64::from(u16::from_be_bytes(
                    self.take(2)?
                        .try_into()
                        .map_err(|_| DecodeError::Truncated)?,
                ));
                if u8::try_from(value).is_ok() {
                    return Err(DecodeError::NonCanonical);
                }
                value
            }
            26 => {
                let value = u64::from(u32::from_be_bytes(
                    self.take(4)?
                        .try_into()
                        .map_err(|_| DecodeError::Truncated)?,
                ));
                if u16::try_from(value).is_ok() {
                    return Err(DecodeError::NonCanonical);
                }
                value
            }
            27 => {
                let value = u64::from_be_bytes(
                    self.take(8)?
                        .try_into()
                        .map_err(|_| DecodeError::Truncated)?,
                );
                if u32::try_from(value).is_ok() {
                    return Err(DecodeError::NonCanonical);
                }
                value
            }
            _ => return Err(DecodeError::UnsupportedType),
        };
        Ok((major, value))
    }

    fn byte(&mut self) -> Result<u8, DecodeError> {
        let value = *self
            .input
            .get(self.position)
            .ok_or(DecodeError::Truncated)?;
        self.position += 1;
        Ok(value)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], DecodeError> {
        let end = self
            .position
            .checked_add(len)
            .ok_or(DecodeError::LimitExceeded)?;
        let value = self
            .input
            .get(self.position..end)
            .ok_or(DecodeError::Truncated)?;
        self.position = end;
        Ok(value)
    }
}
