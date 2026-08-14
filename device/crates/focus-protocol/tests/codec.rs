use focus_protocol::{
    Capability, ClockAnchorRequest, ClockAnchorResponse, DecodeError, EncodeError, ErrorCode,
    ErrorResponse, FirmwareVersion, HelloResponse, JournalHealth, JournalStatus,
    MAX_LOGICAL_MESSAGE_BYTES, MAX_RECORDS_PER_PAGE, PresetId, PresetName, PresetSnapshot,
    ProductName, ProtocolVersion, Request, RequestEnvelope, Response, ResponseEnvelope,
    SessionOutcome, SessionPageRequest, SessionPageResponse, SessionRecord, StatusResponse,
    ViewState, decode_request, decode_response, encode_request, encode_response,
};
use heapless::{String, Vec};

fn bounded<const N: usize>(value: &str) -> String<N> {
    String::try_from(value).expect("test string must fit")
}

fn preset() -> PresetSnapshot {
    PresetSnapshot {
        id: PresetId::try_from("pomodoro").expect("preset ID must fit"),
        name: PresetName::try_from("Pomodoro").expect("preset name must fit"),
        planned_duration_ms: 1_500_000,
    }
}

fn round_trip_request(request: Request) {
    let expected = RequestEnvelope {
        version: ProtocolVersion::CURRENT,
        request_id: 42,
        request,
    };
    let mut output = [0; MAX_LOGICAL_MESSAGE_BYTES];
    let len = encode_request(&expected, &mut output).expect("request must encode");
    assert_eq!(decode_request(&output[..len]), Ok(expected));
}

fn round_trip_response(response: Response) {
    let expected = ResponseEnvelope {
        version: ProtocolVersion::CURRENT,
        request_id: 42,
        response,
    };
    let mut output = [0; MAX_LOGICAL_MESSAGE_BYTES];
    let len = encode_response(&expected, &mut output).expect("response must encode");
    assert_eq!(decode_response(&output[..len]), Ok(expected));
}

#[test]
fn request_messages_round_trip() {
    round_trip_request(Request::Hello);
    round_trip_request(Request::GetStatus);
    round_trip_request(Request::GetSessionPage(SessionPageRequest {
        journal_epoch: Some([0x22; 8]),
        after_sequence: 7,
        limit: 8,
    }));
    round_trip_request(Request::GetSessionPage(SessionPageRequest {
        journal_epoch: None,
        after_sequence: 0,
        limit: 1,
    }));
    round_trip_request(Request::SetClockAnchor(ClockAnchorRequest {
        utc_ms: 1_786_669_200_000,
    }));
    round_trip_request(Request::Unknown { message_kind: 200 });
}

#[test]
fn hello_response_round_trips() {
    let mut capabilities = Vec::new();
    capabilities
        .extend_from_slice(&[
            Capability::ReadStatus,
            Capability::ReadSessionPages,
            Capability::SetClockAnchor,
        ])
        .expect("capabilities must fit");
    round_trip_response(Response::Hello(HelloResponse {
        device_id: [0x11; 16],
        product_name: ProductName::try_from("Focus Timer").expect("product name must fit"),
        firmware_version: FirmwareVersion::try_from("0.1.0").expect("version must fit"),
        supported_version: ProtocolVersion::CURRENT,
        capabilities,
    }));
}

#[test]
fn status_response_round_trips_with_empty_and_populated_bounds() {
    for journal in [
        JournalStatus {
            epoch: [0x33; 8],
            oldest_sequence: None,
            latest_sequence: None,
            health: JournalHealth::Healthy,
        },
        JournalStatus {
            epoch: [0x33; 8],
            oldest_sequence: Some(5),
            latest_sequence: Some(19),
            health: JournalHealth::Degraded,
        },
    ] {
        round_trip_response(Response::Status(StatusResponse {
            view_state: ViewState::Paused,
            preset: preset(),
            remaining_duration_ms: 840_000,
            journal,
            clock_known: true,
        }));
    }
}

#[test]
fn session_page_response_round_trips_nullable_timestamps() {
    let mut records = Vec::new();
    records
        .push(SessionRecord {
            sequence: 5,
            preset: preset(),
            active_duration_ms: 1_500_000,
            outcome: SessionOutcome::Completed,
            started_at_utc_ms: Some(1_786_669_200_000),
            ended_at_utc_ms: Some(1_786_670_700_000),
        })
        .expect("record must fit");
    records
        .push(SessionRecord {
            sequence: 6,
            preset: preset(),
            active_duration_ms: 412_000,
            outcome: SessionOutcome::Cancelled,
            started_at_utc_ms: None,
            ended_at_utc_ms: None,
        })
        .expect("record must fit");

    round_trip_response(Response::SessionPage(SessionPageResponse {
        journal_epoch: [0x44; 8],
        oldest_sequence: Some(5),
        latest_sequence: Some(9),
        gap: true,
        epoch_changed: false,
        records,
        continuation_after_sequence: Some(6),
    }));
}

#[test]
fn clock_and_structured_error_responses_round_trip() {
    round_trip_response(Response::ClockAnchor(ClockAnchorResponse {
        accepted_utc_ms: 1_786_669_200_000,
        device_monotonic_ms_at_receipt: 123_456,
    }));
    round_trip_response(Response::Error(ErrorResponse {
        code: ErrorCode::UnsupportedProtocolVersion,
        failed_message_kind: Some(1),
        field_id: None,
        supported_version: Some(ProtocolVersion::CURRENT),
    }));
    round_trip_response(Response::Error(ErrorResponse {
        code: ErrorCode::InvalidField,
        failed_message_kind: Some(5),
        field_id: Some(2),
        supported_version: None,
    }));
}

#[test]
fn hello_request_has_explicit_canonical_bytes() {
    let request = RequestEnvelope {
        version: ProtocolVersion::CURRENT,
        request_id: 7,
        request: Request::Hello,
    };
    let mut output = [0; 32];
    let len = encode_request(&request, &mut output).expect("request must encode");
    assert_eq!(
        &output[..len],
        &[
            0xa5, 0x00, 0x01, 0x01, 0x00, 0x02, 0x07, 0x03, 0x01, 0x04, 0xa0
        ]
    );
}

#[test]
fn maximum_page_stays_below_logical_message_limit() {
    let max_text = "x".repeat(32);
    let mut records = Vec::new();
    for sequence in 1..=MAX_RECORDS_PER_PAGE {
        records
            .push(SessionRecord {
                sequence: u64::try_from(sequence).expect("sequence must fit"),
                preset: PresetSnapshot {
                    id: bounded(&max_text),
                    name: bounded(&max_text),
                    planned_duration_ms: u32::MAX,
                },
                active_duration_ms: u32::MAX,
                outcome: SessionOutcome::Completed,
                started_at_utc_ms: Some(9_007_199_254_740_990),
                ended_at_utc_ms: Some(9_007_199_254_740_991),
            })
            .expect("record must fit");
    }
    let response = ResponseEnvelope {
        version: ProtocolVersion::CURRENT,
        request_id: u32::MAX,
        response: Response::SessionPage(SessionPageResponse {
            journal_epoch: [0xff; 8],
            oldest_sequence: Some(1),
            latest_sequence: Some(8),
            gap: false,
            epoch_changed: false,
            records,
            continuation_after_sequence: None,
        }),
    };
    let mut output = [0; MAX_LOGICAL_MESSAGE_BYTES];
    let len = encode_response(&response, &mut output).expect("maximum page must encode");
    assert!(len < MAX_LOGICAL_MESSAGE_BYTES);
    assert_eq!(decode_response(&output[..len]), Ok(response));
}

#[test]
fn invalid_models_are_rejected_before_encoding() {
    let invalid_cursor = RequestEnvelope {
        version: ProtocolVersion::CURRENT,
        request_id: 1,
        request: Request::GetSessionPage(SessionPageRequest {
            journal_epoch: None,
            after_sequence: 1,
            limit: 8,
        }),
    };
    let mut output = [0; MAX_LOGICAL_MESSAGE_BYTES];
    assert_eq!(
        encode_request(&invalid_cursor, &mut output),
        Err(EncodeError::InvalidValue("journal_epoch"))
    );

    let zero_request = RequestEnvelope {
        version: ProtocolVersion::CURRENT,
        request_id: 0,
        request: Request::Hello,
    };
    assert_eq!(
        encode_request(&zero_request, &mut output),
        Err(EncodeError::InvalidValue("request_id"))
    );
}

#[test]
fn decoder_rejects_noncanonical_duplicate_and_trailing_input() {
    let noncanonical_request_id = [
        0xa5, 0x00, 0x01, 0x01, 0x00, 0x02, 0x18, 0x07, 0x03, 0x01, 0x04, 0xa0,
    ];
    assert_eq!(
        decode_request(&noncanonical_request_id),
        Err(DecodeError::NonCanonical)
    );

    let duplicate_key = [
        0xa5, 0x00, 0x01, 0x00, 0x00, 0x02, 0x01, 0x03, 0x01, 0x04, 0xa0,
    ];
    assert_eq!(
        decode_request(&duplicate_key),
        Err(DecodeError::DuplicateOrUnorderedKey)
    );

    let trailing = [
        0xa5, 0x00, 0x01, 0x01, 0x00, 0x02, 0x01, 0x03, 0x01, 0x04, 0xa0, 0x00,
    ];
    assert_eq!(decode_request(&trailing), Err(DecodeError::TrailingData));
}

#[test]
fn compatible_unknown_optional_field_is_validated_and_ignored() {
    let with_unknown_boolean = [
        0xa6, 0x00, 0x01, 0x01, 0x00, 0x02, 0x01, 0x03, 0x01, 0x04, 0xa0, 0x05, 0xf5,
    ];
    assert_eq!(
        decode_request(&with_unknown_boolean),
        Ok(RequestEnvelope {
            version: ProtocolVersion::CURRENT,
            request_id: 1,
            request: Request::Hello,
        })
    );

    let with_forbidden_null = [
        0xa6, 0x00, 0x01, 0x01, 0x00, 0x02, 0x01, 0x03, 0x01, 0x04, 0xa0, 0x05, 0xf6,
    ];
    assert_eq!(
        decode_request(&with_forbidden_null),
        Err(DecodeError::UnsupportedType)
    );
}

#[test]
fn buffers_and_inputs_are_bounded() {
    let request = RequestEnvelope {
        version: ProtocolVersion::CURRENT,
        request_id: 1,
        request: Request::Hello,
    };
    assert_eq!(
        encode_request(&request, &mut [0; 4]),
        Err(EncodeError::OutputTooSmall)
    );
    assert_eq!(decode_request(&[]), Err(DecodeError::Empty));
    assert_eq!(
        decode_request(&[0; MAX_LOGICAL_MESSAGE_BYTES + 1]),
        Err(DecodeError::MessageTooLarge)
    );
}
