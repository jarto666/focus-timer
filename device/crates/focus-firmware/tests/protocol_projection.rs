use focus_core::{App, SettingsLoad, default_catalog};
use focus_firmware::protocol_projection::{
    ProjectionError, session_page_response, status_response,
};
use focus_protocol::{JournalHealth as ProtocolJournalHealth, SessionOutcome as ProtocolOutcome};
use focus_sync::{
    Journal, JournalHealth, JournalStatus, PendingRecord, PresetSnapshot, SessionOutcome,
    VolatileClock, project_status,
};
use heapless::String;

const DEVICE_ID: [u8; 16] = [0x11; 16];
const EPOCH: [u8; 8] = [0x22; 8];

fn text<const N: usize>(value: &str) -> String<N> {
    String::try_from(value).unwrap()
}

#[test]
fn immutable_status_is_copied_without_timer_access_or_fabrication() {
    let catalog = default_catalog();
    let app = App::boot(catalog, SettingsLoad::Missing).0;
    let source = project_status(
        &app,
        10,
        JournalStatus {
            epoch: EPOCH,
            oldest_sequence: Some(2),
            latest_sequence: Some(4),
            health: JournalHealth::Degraded,
        },
        VolatileClock::new(),
    );

    let projected = status_response(&source).unwrap();
    assert_eq!(projected.preset.id.as_str(), "pomodoro");
    assert_eq!(projected.remaining_duration_ms, 1_500_000);
    assert_eq!(projected.journal.health, ProtocolJournalHealth::Degraded);
    assert!(!projected.clock_known);
}

#[test]
fn page_projection_preserves_order_outcome_and_nullable_time() {
    let mut journal = Journal::<2>::new(DEVICE_ID, EPOCH).unwrap();
    journal
        .append(PendingRecord {
            preset: PresetSnapshot {
                id: text("focus"),
                name: text("Focus"),
                planned_duration_ms: 3_000_000,
            },
            active_duration_ms: 1_234,
            outcome: SessionOutcome::Cancelled,
            started_at_utc_ms: None,
            ended_at_utc_ms: None,
        })
        .unwrap();
    let page = journal.page::<8>(None, 0, 8).unwrap();

    let projected = session_page_response(&page).unwrap();
    assert_eq!(projected.records[0].sequence, 1);
    assert_eq!(projected.records[0].outcome, ProtocolOutcome::Cancelled);
    assert_eq!(projected.records[0].started_at_utc_ms, None);
    assert_eq!(projected.records[0].ended_at_utc_ms, None);
}

#[test]
fn projection_rejects_values_instead_of_truncating_them() {
    let catalog = default_catalog();
    let app = App::boot(catalog, SettingsLoad::Missing).0;
    let mut source = project_status(
        &app,
        0,
        JournalStatus::unavailable(EPOCH),
        VolatileClock::new(),
    );
    source.remaining_duration_ms = u64::from(u32::MAX) + 1;
    assert_eq!(
        status_response(&source),
        Err(ProjectionError::DurationOutOfRange)
    );
}
