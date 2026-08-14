#![no_std]

//! Bounded logical models and explicit deterministic CBOR codecs for the
//! Focus Timer synchronization protocol.
//!
//! The normative registry lives in `protocol/schema.md` at the repository
//! root. This crate deliberately uses no generic serialization framework: each
//! registered numeric key is written and read explicitly.

mod codec;
mod model;
mod transport;

pub use codec::{
    DecodeError, EncodeError, decode_request, decode_response, encode_request, encode_response,
};
pub use model::{
    Capability, ClockAnchorRequest, ClockAnchorResponse, DeviceId, ErrorCode, ErrorResponse,
    FirmwareVersion, HelloResponse, JournalEpoch, JournalHealth, JournalStatus, MAX_CAPABILITIES,
    MAX_FIRMWARE_VERSION_BYTES, MAX_LOGICAL_MESSAGE_BYTES, MAX_PRESET_ID_BYTES,
    MAX_PRESET_NAME_BYTES, MAX_PRODUCT_NAME_BYTES, MAX_RECORDS_PER_PAGE, MAX_SAFE_INTEGER,
    PresetId, PresetName, PresetSnapshot, ProductName, ProtocolVersion, Request, RequestEnvelope,
    Response, ResponseEnvelope, SessionOutcome, SessionPageRequest, SessionPageResponse,
    SessionRecord, StatusResponse, ViewState,
};
pub use transport::{
    BLE_END_FLAG, BLE_FRAME_HEADER_BYTES, BLE_FRAME_VERSION, BLE_REASSEMBLY_TIMEOUT_MS,
    BLE_START_FLAG, FragmentError, Fragmenter, FrameHeader, OwnedFragmenter, Reassembler,
    ReassemblyError, ReassemblyStatus, crc32_iso_hdlc, next_transfer_id, parse_frame,
};
