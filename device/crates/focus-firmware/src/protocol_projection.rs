//! Lossless bounded projections from device synchronization models to protocol models.

use focus_protocol::{
    JournalHealth as ProtocolJournalHealth, JournalStatus as ProtocolJournalStatus,
    MAX_RECORDS_PER_PAGE, PresetSnapshot as ProtocolPresetSnapshot,
    SessionOutcome as ProtocolSessionOutcome, SessionPageResponse,
    SessionRecord as ProtocolSessionRecord, StatusResponse, ViewState as ProtocolViewState,
};
use focus_sync::{DeviceStatus, JournalHealth, JournalPage, PresetSnapshot, SessionOutcome};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionError {
    DurationOutOfRange,
    PresetTextTooLong,
    PageTooLarge,
}

/// Copies one immutable timer projection into its wire-domain counterpart.
///
/// # Errors
///
/// Rejects values that cannot be represented by the protocol's explicit
/// duration and text limits.
pub fn status_response(status: DeviceStatus) -> Result<StatusResponse, ProjectionError> {
    Ok(StatusResponse {
        view_state: match status.view_state {
            focus_core::ViewState::Idle => ProtocolViewState::Idle,
            focus_core::ViewState::Running => ProtocolViewState::Running,
            focus_core::ViewState::Paused => ProtocolViewState::Paused,
            focus_core::ViewState::Completed => ProtocolViewState::Completed,
        },
        preset: protocol_preset(
            status.preset.id.as_str(),
            status.preset.name,
            status.preset.duration_ms,
        )?,
        remaining_duration_ms: duration(status.remaining_duration_ms)?,
        journal: ProtocolJournalStatus {
            epoch: status.journal.epoch,
            oldest_sequence: status.journal.oldest_sequence,
            latest_sequence: status.journal.latest_sequence,
            health: journal_health(status.journal.health),
        },
        clock_known: status.clock_known,
    })
}

/// Copies one stateless journal page into the fixed protocol page.
///
/// # Errors
///
/// Rejects oversized pages, text, or durations rather than truncating them.
pub fn session_page_response<const N: usize>(
    page: &JournalPage<N>,
) -> Result<SessionPageResponse, ProjectionError> {
    if page.records.len() > MAX_RECORDS_PER_PAGE {
        return Err(ProjectionError::PageTooLarge);
    }

    let mut records = heapless::Vec::new();
    for record in &page.records {
        records
            .push(ProtocolSessionRecord {
                sequence: record.sequence,
                preset: protocol_sync_preset(&record.preset)?,
                active_duration_ms: duration(record.active_duration_ms)?,
                outcome: match record.outcome {
                    SessionOutcome::Completed => ProtocolSessionOutcome::Completed,
                    SessionOutcome::Cancelled => ProtocolSessionOutcome::Cancelled,
                },
                started_at_utc_ms: record.started_at_utc_ms,
                ended_at_utc_ms: record.ended_at_utc_ms,
            })
            .map_err(|_| ProjectionError::PageTooLarge)?;
    }

    Ok(SessionPageResponse {
        journal_epoch: page.journal_epoch,
        oldest_sequence: page.oldest_sequence,
        latest_sequence: page.latest_sequence,
        gap: page.gap,
        epoch_changed: page.epoch_changed,
        records,
        continuation_after_sequence: page.continuation_after_sequence,
    })
}

fn protocol_sync_preset(
    preset: &PresetSnapshot,
) -> Result<ProtocolPresetSnapshot, ProjectionError> {
    protocol_preset(
        preset.id.as_str(),
        preset.name.as_str(),
        preset.planned_duration_ms,
    )
}

fn protocol_preset(
    id: &str,
    name: &str,
    planned_duration_ms: u64,
) -> Result<ProtocolPresetSnapshot, ProjectionError> {
    Ok(ProtocolPresetSnapshot {
        id: id
            .try_into()
            .map_err(|()| ProjectionError::PresetTextTooLong)?,
        name: name
            .try_into()
            .map_err(|()| ProjectionError::PresetTextTooLong)?,
        planned_duration_ms: duration(planned_duration_ms)?,
    })
}

fn duration(value: u64) -> Result<u32, ProjectionError> {
    u32::try_from(value).map_err(|_| ProjectionError::DurationOutOfRange)
}

const fn journal_health(health: JournalHealth) -> ProtocolJournalHealth {
    match health {
        JournalHealth::Healthy => ProtocolJournalHealth::Healthy,
        JournalHealth::Degraded => ProtocolJournalHealth::Degraded,
        JournalHealth::Unavailable => ProtocolJournalHealth::Unavailable,
    }
}
