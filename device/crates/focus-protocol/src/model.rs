use heapless::{String, Vec};

pub const PROTOCOL_MAJOR: u8 = 1;
pub const PROTOCOL_MINOR: u8 = 0;
pub const MAX_LOGICAL_MESSAGE_BYTES: usize = 2_048;
pub const MAX_RECORDS_PER_PAGE: usize = 8;
pub const MAX_CAPABILITIES: usize = 8;
pub const MAX_PRODUCT_NAME_BYTES: usize = 24;
pub const MAX_FIRMWARE_VERSION_BYTES: usize = 32;
pub const MAX_PRESET_ID_BYTES: usize = 32;
pub const MAX_PRESET_NAME_BYTES: usize = 32;
pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

pub type DeviceId = [u8; 16];
pub type JournalEpoch = [u8; 8];
pub type ProductName = String<MAX_PRODUCT_NAME_BYTES>;
pub type FirmwareVersion = String<MAX_FIRMWARE_VERSION_BYTES>;
pub type PresetId = String<MAX_PRESET_ID_BYTES>;
pub type PresetName = String<MAX_PRESET_NAME_BYTES>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolVersion {
    pub major: u8,
    pub minor: u8,
}

impl ProtocolVersion {
    pub const CURRENT: Self = Self {
        major: PROTOCOL_MAJOR,
        minor: PROTOCOL_MINOR,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Capability {
    ReadStatus = 1,
    ReadSessionPages = 2,
    SetClockAnchor = 3,
}

impl Capability {
    pub(crate) const fn from_wire(value: u64) -> Option<Self> {
        match value {
            1 => Some(Self::ReadStatus),
            2 => Some(Self::ReadSessionPages),
            3 => Some(Self::SetClockAnchor),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ViewState {
    Idle = 0,
    Running = 1,
    Paused = 2,
    Completed = 3,
}

impl ViewState {
    pub(crate) const fn from_wire(value: u64) -> Option<Self> {
        match value {
            0 => Some(Self::Idle),
            1 => Some(Self::Running),
            2 => Some(Self::Paused),
            3 => Some(Self::Completed),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum JournalHealth {
    Healthy = 0,
    Degraded = 1,
    Unavailable = 2,
}

impl JournalHealth {
    pub(crate) const fn from_wire(value: u64) -> Option<Self> {
        match value {
            0 => Some(Self::Healthy),
            1 => Some(Self::Degraded),
            2 => Some(Self::Unavailable),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SessionOutcome {
    Completed = 0,
    Cancelled = 1,
}

impl SessionOutcome {
    pub(crate) const fn from_wire(value: u64) -> Option<Self> {
        match value {
            0 => Some(Self::Completed),
            1 => Some(Self::Cancelled),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ErrorCode {
    MalformedEnvelope = 1,
    UnsupportedProtocolVersion = 2,
    UnsupportedMessage = 3,
    MalformedPayload = 4,
    InvalidField = 5,
    LimitExceeded = 6,
    HandshakeRequired = 7,
    Busy = 8,
    JournalUnavailable = 9,
    InternalError = 10,
}

impl ErrorCode {
    pub(crate) const fn from_wire(value: u64) -> Option<Self> {
        match value {
            1 => Some(Self::MalformedEnvelope),
            2 => Some(Self::UnsupportedProtocolVersion),
            3 => Some(Self::UnsupportedMessage),
            4 => Some(Self::MalformedPayload),
            5 => Some(Self::InvalidField),
            6 => Some(Self::LimitExceeded),
            7 => Some(Self::HandshakeRequired),
            8 => Some(Self::Busy),
            9 => Some(Self::JournalUnavailable),
            10 => Some(Self::InternalError),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresetSnapshot {
    pub id: PresetId,
    pub name: PresetName,
    pub planned_duration_ms: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HelloResponse {
    pub device_id: DeviceId,
    pub product_name: ProductName,
    pub firmware_version: FirmwareVersion,
    pub supported_version: ProtocolVersion,
    pub capabilities: Vec<Capability, MAX_CAPABILITIES>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JournalStatus {
    pub epoch: JournalEpoch,
    pub oldest_sequence: Option<u64>,
    pub latest_sequence: Option<u64>,
    pub health: JournalHealth,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusResponse {
    pub view_state: ViewState,
    pub preset: PresetSnapshot,
    pub remaining_duration_ms: u32,
    pub journal: JournalStatus,
    pub clock_known: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionPageRequest {
    pub journal_epoch: Option<JournalEpoch>,
    pub after_sequence: u64,
    pub limit: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionRecord {
    pub sequence: u64,
    pub preset: PresetSnapshot,
    pub active_duration_ms: u32,
    pub outcome: SessionOutcome,
    pub started_at_utc_ms: Option<u64>,
    pub ended_at_utc_ms: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionPageResponse {
    pub journal_epoch: JournalEpoch,
    pub oldest_sequence: Option<u64>,
    pub latest_sequence: Option<u64>,
    pub gap: bool,
    pub epoch_changed: bool,
    pub records: Vec<SessionRecord, MAX_RECORDS_PER_PAGE>,
    pub continuation_after_sequence: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClockAnchorRequest {
    pub utc_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClockAnchorResponse {
    pub accepted_utc_ms: u64,
    pub device_monotonic_ms_at_receipt: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ErrorResponse {
    pub code: ErrorCode,
    pub failed_message_kind: Option<u64>,
    pub field_id: Option<u64>,
    pub supported_version: Option<ProtocolVersion>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Request {
    Hello,
    GetStatus,
    GetSessionPage(SessionPageRequest),
    SetClockAnchor(ClockAnchorRequest),
    Unknown { message_kind: u64 },
}

impl Request {
    pub(crate) const fn message_kind(&self) -> u64 {
        match self {
            Self::Hello => 1,
            Self::GetStatus => 3,
            Self::GetSessionPage(_) => 5,
            Self::SetClockAnchor(_) => 7,
            Self::Unknown { message_kind } => *message_kind,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
// The bounded page is intentionally owned so the no_std codec never returns
// borrowed or heap-allocated protocol data.
#[allow(clippy::large_enum_variant)]
pub enum Response {
    Hello(HelloResponse),
    Status(StatusResponse),
    SessionPage(SessionPageResponse),
    ClockAnchor(ClockAnchorResponse),
    Error(ErrorResponse),
}

impl Response {
    pub(crate) const fn message_kind(&self) -> u64 {
        match self {
            Self::Hello(_) => 2,
            Self::Status(_) => 4,
            Self::SessionPage(_) => 6,
            Self::ClockAnchor(_) => 8,
            Self::Error(_) => 255,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestEnvelope {
    pub version: ProtocolVersion,
    pub request_id: u32,
    pub request: Request,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponseEnvelope {
    pub version: ProtocolVersion,
    pub request_id: u32,
    pub response: Response,
}
