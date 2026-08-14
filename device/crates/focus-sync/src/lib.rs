#![no_std]

//! Host-testable synchronization domain for the device firmware.
//!
//! This crate owns no flash, radio, wall clock, GPIO, or mutable timer state.
//! Firmware adapters provide persistence and transport around these bounded
//! models.

mod clock;
mod journal;
mod status;

pub use clock::{ClockError, VolatileClock};
pub use journal::{
    DeviceId, Journal, JournalEpoch, JournalError, JournalHealth, JournalPage, JournalRecord,
    MAX_JOURNAL_SEQUENCE, MAX_PRESET_ID_BYTES, MAX_PRESET_NAME_BYTES, PendingRecord,
    PresetSnapshot, SessionOutcome,
};
pub use status::{DeviceStatus, JournalStatus, project_status};
