//! Pure protocol-handshake state kept outside BLE callbacks and timer state.

use focus_protocol::{
    ErrorCode, ErrorResponse, HelloResponse, ProtocolVersion, Request, RequestEnvelope, Response,
    ResponseEnvelope, SessionPageRequest,
};

/// Work that the single-owner firmware loop must execute for one valid request.
#[derive(Clone, Debug, Eq, PartialEq)]
// Session-page responses intentionally carry a bounded owned page. Keeping the
// response inline avoids a heap allocator in the no_std adapter.
#[allow(clippy::large_enum_variant)]
pub enum ProtocolAction {
    Respond(ResponseEnvelope),
    ReadStatus {
        request_id: u32,
        version: ProtocolVersion,
    },
    ReadSessionPage {
        request_id: u32,
        version: ProtocolVersion,
        page: SessionPageRequest,
    },
    SetClockAnchor {
        request_id: u32,
        version: ProtocolVersion,
        utc_ms: u64,
    },
}

/// One negotiated logical session. A new BLE connection gets a fresh instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolSession {
    hello: HelloResponse,
    negotiated_version: Option<ProtocolVersion>,
}

impl ProtocolSession {
    #[must_use]
    pub const fn new(hello: HelloResponse) -> Self {
        Self {
            hello,
            negotiated_version: None,
        }
    }

    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.negotiated_version.is_some()
    }

    #[must_use]
    pub const fn negotiated_version(&self) -> Option<ProtocolVersion> {
        self.negotiated_version
    }

    /// Clears all connection-scoped compatibility state after link loss.
    pub const fn reset(&mut self) {
        self.negotiated_version = None;
    }

    /// Validates compatibility and turns one decoded request into bounded work.
    #[must_use]
    pub fn handle(&mut self, envelope: &RequestEnvelope) -> ProtocolAction {
        if envelope.version.major != ProtocolVersion::CURRENT.major {
            return Self::error(
                envelope.request_id,
                ProtocolVersion::CURRENT,
                ErrorCode::UnsupportedProtocolVersion,
                Some(envelope.request.message_kind()),
                Some(ProtocolVersion::CURRENT),
            );
        }

        if matches!(envelope.request, Request::Hello) {
            let version = ProtocolVersion {
                major: ProtocolVersion::CURRENT.major,
                minor: ProtocolVersion::CURRENT.minor,
            };
            self.negotiated_version = Some(version);
            return ProtocolAction::Respond(ResponseEnvelope {
                version,
                request_id: envelope.request_id,
                response: Response::Hello(self.hello.clone()),
            });
        }

        let Some(version) = self.negotiated_version else {
            return Self::error(
                envelope.request_id,
                ProtocolVersion {
                    major: ProtocolVersion::CURRENT.major,
                    minor: ProtocolVersion::CURRENT.minor,
                },
                ErrorCode::HandshakeRequired,
                Some(envelope.request.message_kind()),
                None,
            );
        };

        if envelope.version != version {
            return Self::error(
                envelope.request_id,
                version,
                ErrorCode::UnsupportedProtocolVersion,
                Some(envelope.request.message_kind()),
                Some(ProtocolVersion::CURRENT),
            );
        }

        match &envelope.request {
            Request::Hello => unreachable!("hello is handled before the handshake gate"),
            Request::GetStatus => ProtocolAction::ReadStatus {
                request_id: envelope.request_id,
                version,
            },
            Request::GetSessionPage(page) => ProtocolAction::ReadSessionPage {
                request_id: envelope.request_id,
                version,
                page: *page,
            },
            Request::SetClockAnchor(anchor) => ProtocolAction::SetClockAnchor {
                request_id: envelope.request_id,
                version,
                utc_ms: anchor.utc_ms,
            },
            Request::Unknown { message_kind } => Self::error(
                envelope.request_id,
                version,
                ErrorCode::UnsupportedMessage,
                Some(*message_kind),
                None,
            ),
        }
    }

    fn error(
        request_id: u32,
        version: ProtocolVersion,
        code: ErrorCode,
        failed_message_kind: Option<u64>,
        supported_version: Option<ProtocolVersion>,
    ) -> ProtocolAction {
        ProtocolAction::Respond(ResponseEnvelope {
            version,
            request_id,
            response: Response::Error(ErrorResponse {
                code,
                failed_message_kind,
                field_id: None,
                supported_version,
            }),
        })
    }
}
