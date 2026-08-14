use focus_core::{App, Preset, ViewState};

use crate::{JournalEpoch, JournalHealth, VolatileClock};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JournalStatus {
    pub epoch: JournalEpoch,
    pub oldest_sequence: Option<u64>,
    pub latest_sequence: Option<u64>,
    pub health: JournalHealth,
}

impl JournalStatus {
    #[must_use]
    pub const fn unavailable(epoch: JournalEpoch) -> Self {
        Self {
            epoch,
            oldest_sequence: None,
            latest_sequence: None,
            health: JournalHealth::Unavailable,
        }
    }
}

/// Immutable read model copied out of the single-owner timer event loop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceStatus {
    pub view_state: ViewState,
    pub preset: Preset,
    pub remaining_duration_ms: u64,
    pub journal: JournalStatus,
    pub clock_known: bool,
}

/// Projects read-only state without granting any adapter mutable `App` access.
#[must_use]
pub fn project_status(
    app: &App,
    now_ms: u64,
    journal: JournalStatus,
    clock: VolatileClock,
) -> DeviceStatus {
    let snapshot = app.snapshot(now_ms);
    DeviceStatus {
        view_state: snapshot.state,
        preset: snapshot.preset,
        remaining_duration_ms: snapshot.remaining_ms,
        journal,
        clock_known: clock.is_known(),
    }
}
