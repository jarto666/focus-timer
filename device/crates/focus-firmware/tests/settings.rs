use focus_core::{
    App, Catalog, Diagnostic, InputEvent, PresetId, SessionState, SettingsFallback, SettingsLoad,
    default_catalog,
};
use focus_firmware::settings::{
    FlushError, FlushOutcome, LoadError, QueueOutcome, SAVE_QUIET_PERIOD_MS, SelectionPersistence,
    SettingsStore, load_settings,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FakeError {
    BufferTooSmall,
    WriteFailed,
}

#[derive(Default)]
struct FakeStore {
    record: Option<std::vec::Vec<u8>>,
    writes: usize,
    fail_writes: bool,
}

impl FakeStore {
    fn with_record(record: &[u8]) -> Self {
        Self {
            record: Some(record.to_vec()),
            ..Self::default()
        }
    }
}

impl SettingsStore for FakeStore {
    type Error = FakeError;

    fn read(&mut self, output: &mut [u8]) -> Result<Option<usize>, Self::Error> {
        let Some(record) = self.record.as_deref() else {
            return Ok(None);
        };
        if record.len() > output.len() {
            return Err(FakeError::BufferTooSmall);
        }
        output[..record.len()].copy_from_slice(record);
        Ok(Some(record.len()))
    }

    fn write(&mut self, record: &[u8]) -> Result<(), Self::Error> {
        self.writes += 1;
        if self.fail_writes {
            return Err(FakeError::WriteFailed);
        }
        self.record = Some(record.to_vec());
        Ok(())
    }
}

fn catalog() -> Catalog {
    default_catalog()
}

#[test]
fn successful_save_decodes_and_restores_the_selected_preset() {
    let mut store = FakeStore::default();
    let mut persistence = SelectionPersistence::new(None);
    persistence
        .selection_changed(10, PresetId::new("reading"))
        .unwrap();

    assert_eq!(
        persistence.flush_due(10 + SAVE_QUIET_PERIOD_MS, &mut store),
        Ok(FlushOutcome::Saved(PresetId::new("reading")))
    );
    assert_eq!(store.writes, 1);
    assert_eq!(
        store.record.as_deref(),
        Some(&b"FT\x01\x07reading"[..]),
        "the version-1 record contains only its header and stable preset id"
    );

    let loaded = load_settings(&mut store).unwrap().unwrap();
    let (app, effects) = App::boot(catalog(), loaded.as_core_load());
    assert_eq!(app.selected_preset().id, PresetId::new("reading"));
    assert_eq!(effects.diagnostic, None);
}

#[test]
fn first_boot_without_a_record_uses_the_default() {
    let mut store = FakeStore::default();
    assert_eq!(load_settings(&mut store).unwrap(), None);

    let (app, effects) = App::boot(catalog(), SettingsLoad::Missing);
    assert_eq!(app.selected_preset().id, PresetId::new("pomodoro"));
    assert_eq!(effects.diagnostic, None);
}

#[test]
fn corrupt_bytes_are_detected_and_the_core_can_fall_back() {
    let mut store = FakeStore::with_record(b"not-a-settings-record");
    assert!(matches!(
        load_settings(&mut store),
        Err(LoadError::Decode(_))
    ));

    let (app, effects) = App::boot(catalog(), SettingsLoad::Corrupt);
    assert_eq!(app.selected_preset().id, PresetId::new("pomodoro"));
    assert_eq!(
        effects.diagnostic,
        Some(Diagnostic::SettingsFallback(SettingsFallback::Corrupt))
    );
}

#[test]
fn unsupported_version_is_decoded_then_rejected_by_the_core() {
    let mut store = FakeStore::with_record(b"FT\x02\x05focus");
    let loaded = load_settings(&mut store).unwrap().unwrap();
    assert_eq!(loaded.current_persisted_id(), None);

    let (app, effects) = App::boot(catalog(), loaded.as_core_load());
    assert_eq!(app.selected_preset().id, PresetId::new("pomodoro"));
    assert_eq!(
        effects.diagnostic,
        Some(Diagnostic::SettingsFallback(
            SettingsFallback::UnsupportedVersion
        ))
    );
}

#[test]
fn removed_preset_record_falls_back_without_blocking_start() {
    let mut store = FakeStore::with_record(b"FT\x01\x07removed");
    let loaded = load_settings(&mut store).unwrap().unwrap();
    let (mut app, effects) = App::boot(catalog(), loaded.as_core_load());

    assert_eq!(app.selected_preset().id, PresetId::new("pomodoro"));
    assert_eq!(
        effects.diagnostic,
        Some(Diagnostic::SettingsFallback(
            SettingsFallback::UnknownPreset
        ))
    );
    app.handle(1, InputEvent::Press);
    assert!(matches!(app.session(), SessionState::Running { .. }));
}

#[test]
fn rapid_changes_coalesce_to_one_write_of_the_last_selection() {
    let mut store = FakeStore::default();
    let mut persistence = SelectionPersistence::new(Some("pomodoro"));

    assert_eq!(
        persistence.selection_changed(0, PresetId::new("reading")),
        Ok(QueueOutcome::Scheduled)
    );
    assert_eq!(
        persistence.selection_changed(500, PresetId::new("quick-sprint")),
        Ok(QueueOutcome::Scheduled)
    );
    assert_eq!(
        persistence.flush_due(1_499, &mut store),
        Ok(FlushOutcome::NotDue)
    );
    assert_eq!(
        persistence.flush_due(1_500, &mut store),
        Ok(FlushOutcome::Saved(PresetId::new("quick-sprint")))
    );
    assert_eq!(store.writes, 1);

    let loaded = load_settings(&mut store).unwrap().unwrap();
    let (app, _) = App::boot(catalog(), loaded.as_core_load());
    assert_eq!(app.selected_preset().id, PresetId::new("quick-sprint"));
}

#[test]
fn returning_to_the_persisted_selection_cancels_a_pending_write() {
    let mut store = FakeStore::default();
    let mut persistence = SelectionPersistence::new(Some("pomodoro"));

    persistence
        .selection_changed(0, PresetId::new("reading"))
        .unwrap();
    assert_eq!(
        persistence.selection_changed(100, PresetId::new("pomodoro")),
        Ok(QueueOutcome::AlreadyPersisted)
    );
    assert_eq!(
        persistence.flush_due(10_000, &mut store),
        Ok(FlushOutcome::NotDue)
    );
    assert_eq!(store.writes, 0);
}

#[test]
fn write_failure_is_not_retried_and_cannot_revert_the_active_selection() {
    let mut store = FakeStore {
        fail_writes: true,
        ..FakeStore::default()
    };
    let mut persistence = SelectionPersistence::new(None);
    let (mut app, _) = App::boot(catalog(), SettingsLoad::Missing);
    let effects = app.handle(10, InputEvent::RotateRight);
    let selected = app.selected_preset();
    persistence
        .selection_changed(10, effects.persist_selection.unwrap())
        .unwrap();

    assert_eq!(
        persistence.flush_due(10 + SAVE_QUIET_PERIOD_MS, &mut store),
        Err(FlushError::Storage(FakeError::WriteFailed))
    );
    assert_eq!(
        persistence.flush_due(20_000, &mut store),
        Ok(FlushOutcome::NotDue),
        "a failed write waits for a later selection change instead of wearing flash"
    );
    assert_eq!(app.selected_preset(), selected);

    app.handle(20_001, InputEvent::Press);
    assert!(matches!(
        app.session(),
        SessionState::Running { active, .. } if active == selected
    ));
}
