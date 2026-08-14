#![no_std]

//! Host-testable synchronization domain for the device firmware.
//!
//! This crate owns no flash, radio, wall clock, GPIO, or mutable timer state.
//! Firmware adapters provide persistence and transport around these bounded
//! models.

mod clock;
mod journal;
mod persistence;
mod status;
mod storage;

pub use clock::{ClockError, VolatileClock};
pub use journal::{
    DeviceId, Journal, JournalEpoch, JournalError, JournalHealth, JournalPage, JournalRecord,
    MAX_JOURNAL_SEQUENCE, MAX_PRESET_ID_BYTES, MAX_PRESET_NAME_BYTES, PendingRecord,
    PresetSnapshot, SessionOutcome,
};
pub use persistence::{
    AppendError, BlobStore, EntropySource, EpochInit, IdentityInit, InitError, InitReport,
    MetadataInit, PersistentJournal, SYNC_NAMESPACE,
};
pub use status::{DeviceStatus, JournalStatus, project_status};
pub use storage::{
    IDENTITY_RECORD_BYTES, JOURNAL_CAPACITY, MAX_ENCODED_JOURNAL_RECORD_BYTES,
    METADATA_RECORD_BYTES, MetadataRecord, StorageDecodeError, StorageEncodeError,
    StoredJournalRecord, decode_identity, decode_metadata, decode_record, encode_identity,
    encode_metadata, encode_record,
};
