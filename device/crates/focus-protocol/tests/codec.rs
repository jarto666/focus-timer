use focus_protocol::{
    Capability, CatalogEntry, CatalogResult, ClockAnchorRequest, ClockAnchorResponse, DecodeError,
    DeviceEvent, EncodeError, ErrorCode, ErrorResponse, EventEnvelope, FirmwareVersion,
    HelloResponse, JournalHealth, JournalStatus, MAX_LOGICAL_MESSAGE_BYTES, MAX_RECORDS_PER_PAGE,
    PresetCatalogResponse, PresetCatalogResultEvent, PresetId, PresetName, PresetSnapshot,
    ProductName, ProposePresetCatalogRequest, ProposePresetCatalogResponse, ProtocolVersion,
    Request, RequestEnvelope, Response, ResponseEnvelope, SessionOutcome, SessionPageRequest,
    SessionPageResponse, SessionRecord, StatusResponse, ViewState, decode_event, decode_request,
    decode_response, encode_event, encode_request, encode_response,
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

fn round_trip_event(event: DeviceEvent) {
    let expected = EventEnvelope {
        version: ProtocolVersion::CURRENT,
        event,
    };
    let mut output = [0; MAX_LOGICAL_MESSAGE_BYTES];
    let len = encode_event(&expected, &mut output).expect("event must encode");
    assert_eq!(decode_event(&output[..len]), Ok(expected));
}

fn custom_preset(id: &str, name: &str, minutes: u32) -> PresetSnapshot {
    PresetSnapshot {
        id: PresetId::try_from(id).expect("preset ID must fit"),
        name: PresetName::try_from(name).expect("preset name must fit"),
        planned_duration_ms: minutes * 60_000,
    }
}

fn catalog() -> PresetCatalogResponse {
    let built_ins = [
        ("deep-work", "Deep Work", 90),
        ("focus", "Focus", 50),
        ("pomodoro", "Pomodoro", 25),
        ("reading", "Reading", 45),
        ("quick-sprint", "Quick Sprint", 15),
    ];
    let mut entries = Vec::new();
    for (id, name, minutes) in built_ins {
        entries
            .push(CatalogEntry {
                preset: custom_preset(id, name, minutes),
                built_in: true,
            })
            .expect("catalog must fit");
    }
    entries
        .push(CatalogEntry {
            preset: custom_preset("writing", "Writing", 45),
            built_in: false,
        })
        .expect("catalog must fit");
    PresetCatalogResponse {
        revision: 7,
        entries,
    }
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
    round_trip_request(Request::GetPresetCatalog);
    let mut custom_entries = Vec::new();
    custom_entries
        .push(custom_preset("writing", "Writing", 45))
        .expect("proposal must fit");
    round_trip_request(Request::ProposePresetCatalog(ProposePresetCatalogRequest {
        expected_revision: 7,
        proposal_id: 99,
        custom_entries,
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
            status_epoch: None,
            status_revision: None,
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
fn catalog_responses_and_events_round_trip() {
    round_trip_response(Response::PresetCatalog(catalog()));
    round_trip_response(Response::ProposePresetCatalog(
        ProposePresetCatalogResponse {
            proposal_id: 99,
            expires_in_ms: 15_000,
        },
    ));

    round_trip_event(DeviceEvent::LiveStatus(StatusResponse {
        view_state: ViewState::Running,
        preset: preset(),
        remaining_duration_ms: 1_234_000,
        journal: JournalStatus {
            epoch: [0x33; 8],
            oldest_sequence: Some(5),
            latest_sequence: Some(19),
            health: JournalHealth::Healthy,
        },
        clock_known: true,
        status_epoch: Some([0x55; 8]),
        status_revision: Some(41),
    }));
    round_trip_event(DeviceEvent::PresetCatalogResult(PresetCatalogResultEvent {
        proposal_id: 99,
        result: CatalogResult::Committed,
        catalog_revision: Some(8),
    }));
    round_trip_event(DeviceEvent::PresetCatalogResult(PresetCatalogResultEvent {
        proposal_id: 100,
        result: CatalogResult::Rejected,
        catalog_revision: None,
    }));
}

#[test]
fn hello_request_has_explicit_canonical_bytes() {
    let request = RequestEnvelope {
        version: ProtocolVersion { major: 1, minor: 0 },
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

    let invalid_event = EventEnvelope {
        version: ProtocolVersion::CURRENT,
        event: DeviceEvent::LiveStatus(StatusResponse {
            view_state: ViewState::Idle,
            preset: preset(),
            remaining_duration_ms: 1_500_000,
            journal: JournalStatus {
                epoch: [1; 8],
                oldest_sequence: None,
                latest_sequence: None,
                health: JournalHealth::Healthy,
            },
            clock_known: false,
            status_epoch: None,
            status_revision: None,
        }),
    };
    assert_eq!(
        encode_event(&invalid_event, &mut output),
        Err(EncodeError::InvalidValue("status_epoch"))
    );

    let mut invalid_custom_entries = Vec::new();
    invalid_custom_entries
        .push(custom_preset("almost", "Almost", 1))
        .expect("proposal must fit");
    invalid_custom_entries[0].planned_duration_ms = 60_001;
    let invalid_proposal = RequestEnvelope {
        version: ProtocolVersion::CURRENT,
        request_id: 1,
        request: Request::ProposePresetCatalog(ProposePresetCatalogRequest {
            expected_revision: 0,
            proposal_id: 1,
            custom_entries: invalid_custom_entries,
        }),
    };
    assert_eq!(
        encode_request(&invalid_proposal, &mut output),
        Err(EncodeError::InvalidValue("planned_duration_ms"))
    );
}

#[test]
fn event_decoder_requires_minor_one_and_reserved_zero_request_id() {
    let event = EventEnvelope {
        version: ProtocolVersion::CURRENT,
        event: DeviceEvent::PresetCatalogResult(PresetCatalogResultEvent {
            proposal_id: 7,
            result: CatalogResult::Expired,
            catalog_revision: None,
        }),
    };
    let mut output = [0; MAX_LOGICAL_MESSAGE_BYTES];
    let len = encode_event(&event, &mut output).expect("event must encode");

    let request_id_offset = output[..len]
        .windows(4)
        .position(|window| window == [0x01, 0x02, 0x00, 0x03])
        .expect("canonical envelope prefix must contain request ID");
    let request_id_byte = request_id_offset + 2;
    output[request_id_byte] = 1;
    assert_eq!(
        decode_event(&output[..len]),
        Err(DecodeError::InvalidValue(2))
    );

    output[request_id_byte] = 0;
    output[4] = 0;
    assert_eq!(
        decode_event(&output[..len]),
        Err(DecodeError::UnsupportedMessage(14))
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
            version: ProtocolVersion { major: 1, minor: 0 },
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
