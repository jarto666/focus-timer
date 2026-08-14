use focus_firmware::protocol_session::{ProtocolAction, ProtocolSession};
use focus_protocol::{
    Capability, ErrorCode, HelloResponse, ProtocolVersion, Request, RequestEnvelope, Response,
    SessionPageRequest,
};
use heapless::{String, Vec};

fn session() -> ProtocolSession {
    let mut product_name = String::new();
    product_name.push_str("FocusTimer").unwrap();
    let mut firmware_version = String::new();
    firmware_version.push_str("0.1.0").unwrap();
    let mut capabilities = Vec::new();
    capabilities.push(Capability::ReadStatus).unwrap();
    capabilities.push(Capability::ReadSessionPages).unwrap();
    capabilities.push(Capability::SetClockAnchor).unwrap();
    ProtocolSession::new(HelloResponse {
        device_id: [0x11; 16],
        product_name,
        firmware_version,
        supported_version: ProtocolVersion::CURRENT,
        capabilities,
    })
}

fn request(request_id: u32, version: ProtocolVersion, request: Request) -> RequestEnvelope {
    RequestEnvelope {
        version,
        request_id,
        request,
    }
}

#[test]
fn hello_negotiates_minor_and_exposes_bounded_identity() {
    let mut session = session();
    let action = session.handle(&request(
        7,
        ProtocolVersion { major: 1, minor: 9 },
        Request::Hello,
    ));

    let ProtocolAction::Respond(response) = action else {
        panic!("hello must respond synchronously");
    };
    assert_eq!(response.request_id, 7);
    assert_eq!(response.version, ProtocolVersion::CURRENT);
    let Response::Hello(hello) = response.response else {
        panic!("hello must return identity");
    };
    assert_eq!(hello.device_id, [0x11; 16]);
    assert_eq!(
        hello.capabilities.as_slice(),
        &[
            Capability::ReadStatus,
            Capability::ReadSessionPages,
            Capability::SetClockAnchor
        ]
    );
    assert!(session.is_ready());
}

#[test]
fn major_mismatch_is_correlated_and_does_not_unlock_requests() {
    let mut session = session();
    let action = session.handle(&request(
        41,
        ProtocolVersion { major: 2, minor: 0 },
        Request::Hello,
    ));
    let ProtocolAction::Respond(response) = action else {
        panic!("mismatch must respond");
    };
    let Response::Error(error) = response.response else {
        panic!("mismatch must be structured");
    };
    assert_eq!(response.request_id, 41);
    assert_eq!(error.code, ErrorCode::UnsupportedProtocolVersion);
    assert_eq!(error.supported_version, Some(ProtocolVersion::CURRENT));
    assert!(!session.is_ready());
}

#[test]
fn non_hello_is_rejected_until_each_connection_handshakes() {
    let mut session = session();
    let action = session.handle(&request(1, ProtocolVersion::CURRENT, Request::GetStatus));
    let ProtocolAction::Respond(response) = action else {
        panic!("pre-handshake request must respond with an error");
    };
    let Response::Error(error) = response.response else {
        panic!("pre-handshake request must be structured");
    };
    assert_eq!(error.code, ErrorCode::HandshakeRequired);

    let _ = session.handle(&request(2, ProtocolVersion::CURRENT, Request::Hello));
    assert!(session.is_ready());
    session.reset();
    assert!(!session.is_ready());
}

#[test]
fn ready_session_dispatches_only_the_read_or_clock_work() {
    let mut session = session();
    let _ = session.handle(&request(1, ProtocolVersion::CURRENT, Request::Hello));

    assert_eq!(
        session.handle(&request(2, ProtocolVersion::CURRENT, Request::GetStatus)),
        ProtocolAction::ReadStatus {
            request_id: 2,
            version: ProtocolVersion::CURRENT,
        }
    );
    assert_eq!(
        session.handle(&request(
            3,
            ProtocolVersion::CURRENT,
            Request::GetSessionPage(SessionPageRequest {
                journal_epoch: None,
                after_sequence: 0,
                limit: 8,
            }),
        )),
        ProtocolAction::ReadSessionPage {
            request_id: 3,
            version: ProtocolVersion::CURRENT,
            page: SessionPageRequest {
                journal_epoch: None,
                after_sequence: 0,
                limit: 8,
            },
        }
    );
}

#[test]
fn unknown_request_does_not_poison_the_following_request() {
    let mut session = session();
    let _ = session.handle(&request(1, ProtocolVersion::CURRENT, Request::Hello));
    let action = session.handle(&request(
        2,
        ProtocolVersion::CURRENT,
        Request::Unknown { message_kind: 91 },
    ));
    let ProtocolAction::Respond(response) = action else {
        panic!("unknown request must respond");
    };
    let Response::Error(error) = response.response else {
        panic!("unknown request must be structured");
    };
    assert_eq!(error.code, ErrorCode::UnsupportedMessage);
    assert_eq!(error.failed_message_kind, Some(91));

    assert!(matches!(
        session.handle(&request(3, ProtocolVersion::CURRENT, Request::GetStatus)),
        ProtocolAction::ReadStatus { request_id: 3, .. }
    ));
}
