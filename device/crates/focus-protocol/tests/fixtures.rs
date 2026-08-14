use focus_protocol::{
    Capability, ClockAnchorRequest, ClockAnchorResponse, ErrorCode, ErrorResponse, FirmwareVersion,
    HelloResponse, JournalHealth, JournalStatus, MAX_LOGICAL_MESSAGE_BYTES, PresetId, PresetName,
    PresetSnapshot, ProductName, ProtocolVersion, Request, RequestEnvelope, Response,
    ResponseEnvelope, SessionOutcome, SessionPageRequest, SessionPageResponse, SessionRecord,
    StatusResponse, ViewState, decode_request, decode_response, encode_request, encode_response,
};
use heapless::Vec as HeaplessVec;

const FIXTURES: &str = include_str!("../../../../protocol/fixtures/logical-v1.txt");

fn fixture(name: &str, expected_direction: &str) -> Vec<u8> {
    let line = FIXTURES
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .find(|line| line.split_whitespace().next() == Some(name))
        .expect("fixture must exist");
    let mut fields = line.split_whitespace();
    assert_eq!(fields.next(), Some(name));
    assert_eq!(fields.next(), Some(expected_direction));
    let hex = fields.next().expect("fixture hex must exist");
    assert_eq!(fields.next(), None);
    assert_eq!(hex.len() % 2, 0);
    (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).expect("fixture must be hex"))
        .collect()
}

fn assert_request(name: &str, request_id: u32, request: Request) {
    let expected = RequestEnvelope {
        version: ProtocolVersion::CURRENT,
        request_id,
        request,
    };
    let fixture = fixture(name, "request");
    let mut output = [0; MAX_LOGICAL_MESSAGE_BYTES];
    let length = encode_request(&expected, &mut output).expect("fixture request must encode");
    assert_eq!(&output[..length], fixture);
    assert_eq!(decode_request(&fixture), Ok(expected));
}

fn assert_response(name: &str, request_id: u32, response: Response) {
    let expected = ResponseEnvelope {
        version: ProtocolVersion::CURRENT,
        request_id,
        response,
    };
    let fixture = fixture(name, "response");
    let mut output = [0; MAX_LOGICAL_MESSAGE_BYTES];
    let length = encode_response(&expected, &mut output).expect("fixture response must encode");
    assert_eq!(&output[..length], fixture);
    assert_eq!(decode_response(&fixture), Ok(expected));
}

fn preset() -> PresetSnapshot {
    PresetSnapshot {
        id: PresetId::try_from("pomodoro").expect("preset ID must fit"),
        name: PresetName::try_from("Pomodoro").expect("preset name must fit"),
        planned_duration_ms: 1_500_000,
    }
}

fn edge_preset() -> PresetSnapshot {
    PresetSnapshot {
        id: PresetId::try_from("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").expect("preset ID must fit"),
        name: PresetName::try_from("yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy")
            .expect("preset name must fit"),
        planned_duration_ms: u32::MAX,
    }
}

#[test]
fn request_fixtures_are_byte_identical_and_historically_decodable() {
    assert_request("hello-request-min", 1, Request::Hello);
    assert_request("status-request-max-id", u32::MAX, Request::GetStatus);
    assert_request(
        "session-page-request-first",
        4,
        Request::GetSessionPage(SessionPageRequest {
            journal_epoch: None,
            after_sequence: 0,
            limit: 1,
        }),
    );
    assert_request(
        "session-page-request-max-cursor",
        5,
        Request::GetSessionPage(SessionPageRequest {
            journal_epoch: Some([0xcc; 8]),
            after_sequence: 9_007_199_254_740_991,
            limit: 8,
        }),
    );
    assert_request(
        "clock-anchor-request-max",
        8,
        Request::SetClockAnchor(ClockAnchorRequest {
            utc_ms: 9_007_199_254_740_991,
        }),
    );
}

#[test]
fn response_fixtures_are_byte_identical_and_historically_decodable() {
    let mut capabilities = HeaplessVec::new();
    capabilities
        .extend_from_slice(&[
            Capability::ReadStatus,
            Capability::ReadSessionPages,
            Capability::SetClockAnchor,
        ])
        .expect("capabilities must fit");
    assert_response(
        "hello-response-full",
        1,
        Response::Hello(HelloResponse {
            device_id: [
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f,
            ],
            product_name: ProductName::try_from("Focus Timer").expect("product name must fit"),
            firmware_version: FirmwareVersion::try_from("0.1.0").expect("version must fit"),
            supported_version: ProtocolVersion::CURRENT,
            capabilities,
        }),
    );
    assert_response(
        "status-response-empty-journal",
        2,
        Response::Status(StatusResponse {
            view_state: ViewState::Completed,
            preset: preset(),
            remaining_duration_ms: 0,
            journal: JournalStatus {
                epoch: [0xaa; 8],
                oldest_sequence: None,
                latest_sequence: None,
                health: JournalHealth::Healthy,
            },
            clock_known: false,
        }),
    );
    assert_response(
        "status-response-max-bounds",
        3,
        Response::Status(StatusResponse {
            view_state: ViewState::Paused,
            preset: edge_preset(),
            remaining_duration_ms: u32::MAX,
            journal: JournalStatus {
                epoch: [0xbb; 8],
                oldest_sequence: Some(9_007_199_254_740_990),
                latest_sequence: Some(9_007_199_254_740_991),
                health: JournalHealth::Degraded,
            },
            clock_known: true,
        }),
    );
}

#[test]
fn page_clock_and_error_fixtures_are_byte_identical_and_historically_decodable() {
    assert_response(
        "session-page-response-empty",
        6,
        Response::SessionPage(SessionPageResponse {
            journal_epoch: [0xdd; 8],
            oldest_sequence: None,
            latest_sequence: None,
            gap: false,
            epoch_changed: true,
            records: HeaplessVec::new(),
            continuation_after_sequence: None,
        }),
    );

    let mut records = HeaplessVec::new();
    for sequence in 1_u64..=8 {
        let (started_at_utc_ms, ended_at_utc_ms) = match sequence {
            1 => (Some(0), Some(1)),
            2 => (None, Some(9_007_199_254_740_991)),
            3 => (Some(9_007_199_254_740_991), None),
            _ => (None, None),
        };
        records
            .push(SessionRecord {
                sequence,
                preset: edge_preset(),
                active_duration_ms: u32::MAX,
                outcome: if sequence % 2 == 1 {
                    SessionOutcome::Completed
                } else {
                    SessionOutcome::Cancelled
                },
                started_at_utc_ms,
                ended_at_utc_ms,
            })
            .expect("record must fit");
    }
    assert_response(
        "session-page-response-edge",
        7,
        Response::SessionPage(SessionPageResponse {
            journal_epoch: [0xee; 8],
            oldest_sequence: Some(1),
            latest_sequence: Some(9),
            gap: false,
            epoch_changed: false,
            records,
            continuation_after_sequence: Some(8),
        }),
    );
    assert_response(
        "clock-anchor-response-edges",
        8,
        Response::ClockAnchor(ClockAnchorResponse {
            accepted_utc_ms: 0,
            device_monotonic_ms_at_receipt: 9_007_199_254_740_991,
        }),
    );
    assert_response(
        "error-response-version",
        9,
        Response::Error(ErrorResponse {
            code: ErrorCode::UnsupportedProtocolVersion,
            failed_message_kind: Some(1),
            field_id: None,
            supported_version: Some(ProtocolVersion::CURRENT),
        }),
    );
    assert_response(
        "error-response-field",
        10,
        Response::Error(ErrorResponse {
            code: ErrorCode::InvalidField,
            failed_message_kind: Some(5),
            field_id: Some(2),
            supported_version: None,
        }),
    );
}
