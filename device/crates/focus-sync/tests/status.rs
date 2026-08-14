use focus_core::{App, Catalog, DEFAULT_PRESETS, SettingsLoad, ViewState};
use focus_sync::{JournalHealth, JournalStatus, VolatileClock, project_status};

const EPOCH: [u8; 8] = [0x22; 8];

fn app() -> App {
    let catalog = Catalog::new(&DEFAULT_PRESETS, 2).unwrap();
    App::boot(catalog, SettingsLoad::Missing).0
}

#[test]
fn projection_copies_timer_journal_and_clock_state() {
    let app = app();
    let mut clock = VolatileClock::new();
    clock.set_anchor(1_700_000_000_000, 10).unwrap();
    let journal = JournalStatus {
        epoch: EPOCH,
        oldest_sequence: Some(4),
        latest_sequence: Some(7),
        health: JournalHealth::Degraded,
    };

    let status = project_status(&app, 50, journal, clock);
    assert_eq!(status.view_state, ViewState::Idle);
    assert_eq!(status.preset, app.selected_preset());
    assert_eq!(
        status.remaining_duration_ms,
        app.selected_preset().duration_ms
    );
    assert_eq!(status.journal, journal);
    assert!(status.clock_known);
}

#[test]
fn unavailable_journal_and_unknown_clock_are_explicit() {
    let app = app();
    let status = project_status(
        &app,
        0,
        JournalStatus::unavailable(EPOCH),
        VolatileClock::new(),
    );

    assert_eq!(status.journal.health, JournalHealth::Unavailable);
    assert_eq!(status.journal.oldest_sequence, None);
    assert_eq!(status.journal.latest_sequence, None);
    assert!(!status.clock_known);
}
