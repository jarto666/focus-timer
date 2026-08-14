use focus_core::{
    App, Catalog, CatalogError, Diagnostic, Effects, FeedbackPattern, InputEvent,
    MAX_SESSION_DURATION_MS, Preset, SETTINGS_SCHEMA_VERSION, SessionOutcome, SessionOutcomeKind,
    SessionState, SettingsFallback, SettingsLoad, SettingsRecord, ViewState, default_catalog,
    default_presets, restore_selection,
};

fn catalog() -> Catalog {
    default_catalog()
}

fn boot() -> App {
    App::boot(catalog(), SettingsLoad::Missing).0
}

#[test]
fn default_catalog_has_required_presets_in_order() {
    let catalog = catalog();
    let expected = [
        ("deep-work", "Deep Work", 90),
        ("focus", "Focus", 50),
        ("pomodoro", "Pomodoro", 25),
        ("reading", "Reading", 45),
        ("quick-sprint", "Quick Sprint", 15),
    ];

    for (index, (id, name, minutes)) in expected.into_iter().enumerate() {
        let preset = catalog.preset(index);
        assert_eq!(preset.id.as_str(), id);
        assert_eq!(preset.name, name);
        assert_eq!(preset.duration_ms, minutes * 60_000);
    }
}

#[test]
fn catalog_rejects_empty_and_bad_default() {
    assert_eq!(Catalog::new(&[], 0).unwrap_err(), CatalogError::Empty);
    assert_eq!(
        Catalog::new(default_presets().as_slice(), default_presets().len()).unwrap_err(),
        CatalogError::InvalidDefaultIndex
    );
}

#[test]
fn catalog_rejects_invalid_entries() {
    let blank_id = [Preset::new("  ", "Name", 1)];
    let blank_name = [Preset::new("id", "\t", 1)];
    let zero = [Preset::new("id", "Name", 0)];
    let too_long = [Preset::new("id", "Name", MAX_SESSION_DURATION_MS + 1)];
    let duplicate = [Preset::new("id", "One", 1), Preset::new("id", "Two", 2)];

    assert_eq!(
        Catalog::new(&blank_id, 0).unwrap_err(),
        CatalogError::BlankId { index: 0 }
    );
    assert_eq!(
        Catalog::new(&blank_name, 0).unwrap_err(),
        CatalogError::BlankName { index: 0 }
    );
    assert_eq!(
        Catalog::new(&zero, 0).unwrap_err(),
        CatalogError::ZeroDuration { index: 0 }
    );
    assert_eq!(
        Catalog::new(&too_long, 0).unwrap_err(),
        CatalogError::DurationOutOfRange { index: 0 }
    );
    assert_eq!(
        Catalog::new(&duplicate, 0).unwrap_err(),
        CatalogError::DuplicateId {
            first: 0,
            duplicate: 1
        }
    );
}

#[test]
fn idle_selection_wraps_and_requests_persistence() {
    let (mut app, _) = App::boot(catalog(), SettingsLoad::Missing);
    assert_eq!(app.selected_preset().id.as_str(), "pomodoro");

    let left = app.handle(0, InputEvent::RotateLeft);
    assert_eq!(app.selected_preset().id.as_str(), "focus");
    assert_eq!(left.persist_selection, Some(app.selected_preset().id));

    app.handle(0, InputEvent::RotateLeft);
    let wrapped = app.handle(0, InputEvent::RotateLeft);
    assert_eq!(app.selected_preset().id.as_str(), "quick-sprint");
    assert_eq!(wrapped.persist_selection, Some(app.selected_preset().id));

    let right = app.handle(0, InputEvent::RotateRight);
    assert_eq!(app.selected_preset().id.as_str(), "deep-work");
    assert!(right.render);
}

#[test]
fn one_press_starts_an_immutable_preset_snapshot() {
    let mut app = boot();
    let selected = app.selected_preset();
    let effects = app.handle(1_000, InputEvent::Press);

    assert_eq!(effects.feedback, Some(FeedbackPattern::Start));
    assert!(effects.render);
    assert!(matches!(
        app.session(),
        SessionState::Running { active, .. } if active == selected
    ));

    app.handle(1_001, InputEvent::RotateRight);
    assert_eq!(app.selected_preset(), selected);
    assert!(matches!(
        app.session(),
        SessionState::Running { active, .. } if active == selected
    ));
}

#[test]
fn running_session_pauses_and_resumes_without_counting_paused_time() {
    let mut app = boot();
    app.handle(1_000, InputEvent::Press);
    app.handle(6_000, InputEvent::Press);

    let paused = app.snapshot(100_000);
    assert_eq!(paused.state, ViewState::Paused);
    assert_eq!(paused.remaining_ms, 25 * 60_000 - 5_000);

    app.handle(100_000, InputEvent::Press);
    let resumed = app.snapshot(100_000);
    assert_eq!(resumed.state, ViewState::Running);
    assert_eq!(resumed.remaining_ms, paused.remaining_ms);
}

#[test]
fn long_press_cancels_running_and_paused_sessions() {
    let mut app = boot();
    app.handle(0, InputEvent::Press);
    assert!(app.handle(1, InputEvent::LongPress).render);
    assert_eq!(app.session(), SessionState::Idle);

    app.handle(2, InputEvent::Press);
    app.handle(3, InputEvent::Press);
    assert!(matches!(app.session(), SessionState::Paused { .. }));
    app.handle(4, InputEvent::LongPress);
    assert_eq!(app.session(), SessionState::Idle);
}

#[test]
fn running_and_paused_cancellation_emit_one_committed_outcome() {
    let mut running = boot();
    let preset = running.selected_preset();
    running.handle(1_000, InputEvent::Press);
    let outcome = running.handle(6_000, InputEvent::LongPress).outcome;
    assert_eq!(
        outcome,
        Some(SessionOutcome {
            kind: SessionOutcomeKind::Cancelled,
            preset: preset.clone(),
            planned_duration_ms: preset.duration_ms,
            active_duration_ms: 5_000,
        })
    );
    assert_eq!(running.handle(6_001, InputEvent::LongPress).outcome, None);

    let mut paused = boot();
    paused.handle(10_000, InputEvent::Press);
    paused.handle(17_000, InputEvent::Press);
    let outcome = paused.handle(99_000, InputEvent::LongPress).outcome;
    assert_eq!(
        outcome,
        Some(SessionOutcome {
            kind: SessionOutcomeKind::Cancelled,
            preset: preset.clone(),
            planned_duration_ms: preset.duration_ms,
            active_duration_ms: 7_000,
        })
    );
}

#[test]
fn countdown_uses_deadline_and_clamps_at_zero() {
    let mut app = boot();
    let duration = app.selected_preset().duration_ms;
    app.handle(10, InputEvent::Press);

    assert_eq!(app.snapshot(1_010).remaining_ms, duration - 1_000);
    let completion = app.handle(10 + duration + 500, InputEvent::TimeAdvanced);
    assert_eq!(completion.feedback, Some(FeedbackPattern::Complete));
    assert_eq!(app.snapshot(u64::MAX).remaining_ms, 0);
    assert_eq!(app.snapshot(u64::MAX).state, ViewState::Completed);
}

#[test]
fn completion_effect_is_emitted_exactly_once() {
    let mut app = boot();
    let duration = app.selected_preset().duration_ms;
    app.handle(0, InputEvent::Press);

    let first = app.handle(duration, InputEvent::TimeAdvanced);
    let later = app.handle(duration + 1, InputEvent::TimeAdvanced);
    assert_eq!(first.feedback, Some(FeedbackPattern::Complete));
    assert_eq!(
        first.outcome,
        Some(SessionOutcome {
            kind: SessionOutcomeKind::Completed,
            preset: app.selected_preset(),
            planned_duration_ms: duration,
            active_duration_ms: duration,
        })
    );
    assert_eq!(later.feedback, None);
    assert_eq!(later.outcome, None);
    assert!(matches!(app.session(), SessionState::Completed { .. }));
}

#[test]
fn intermediate_and_non_session_inputs_emit_no_outcome() {
    let mut app = boot();

    assert_eq!(app.handle(0, InputEvent::RotateLeft).outcome, None);
    assert_eq!(app.handle(0, InputEvent::RotateRight).outcome, None);
    assert_eq!(app.handle(1, InputEvent::TimeAdvanced).outcome, None);
    assert_eq!(app.handle(2, InputEvent::LongPress).outcome, None);

    assert_eq!(app.handle(10, InputEvent::Press).outcome, None);
    assert_eq!(app.handle(20, InputEvent::Press).outcome, None);
    assert_eq!(app.handle(1_000, InputEvent::TimeAdvanced).outcome, None);
    assert_eq!(app.handle(2_000, InputEvent::RotateLeft).outcome, None);
    assert_eq!(app.handle(2_000, InputEvent::Press).outcome, None);

    let completion_at = 2_000 + app.snapshot(2_000).remaining_ms;
    assert!(
        app.handle(completion_at, InputEvent::TimeAdvanced)
            .outcome
            .is_some()
    );
    assert_eq!(
        app.handle(completion_at + 1, InputEvent::TimeAdvanced)
            .outcome,
        None
    );
    assert_eq!(
        app.handle(completion_at + 2, InputEvent::Press).outcome,
        None
    );
}

#[test]
fn completion_press_returns_to_idle_with_selection_retained() {
    let mut app = boot();
    let selected = app.selected_preset();
    app.handle(0, InputEvent::Press);
    app.handle(selected.duration_ms, InputEvent::TimeAdvanced);
    app.handle(selected.duration_ms + 1, InputEvent::Press);

    assert_eq!(app.session(), SessionState::Idle);
    assert_eq!(app.selected_preset(), selected);
}

#[test]
fn time_jump_at_deadline_wins_over_simultaneous_input() {
    let mut app = boot();
    let duration = app.selected_preset().duration_ms;
    app.handle(0, InputEvent::Press);
    let effects = app.handle(duration, InputEvent::Press);

    assert_eq!(effects.feedback, Some(FeedbackPattern::Complete));
    assert!(matches!(app.session(), SessionState::Completed { .. }));
}

#[test]
fn unsupported_events_are_safe_no_ops_in_every_state() {
    let mut app = boot();
    let idle = app.session();
    for event in [InputEvent::LongPress, InputEvent::TimeAdvanced] {
        assert_eq!(app.handle(0, event), Effects::default());
        assert_eq!(app.session(), idle);
    }

    app.handle(1, InputEvent::Press);
    let running = app.session();
    for event in [InputEvent::RotateLeft, InputEvent::RotateRight] {
        assert_eq!(app.handle(1, event), Effects::default());
        assert_eq!(app.session(), running);
    }

    app.handle(2, InputEvent::Press);
    let paused = app.session();
    for event in [
        InputEvent::RotateLeft,
        InputEvent::RotateRight,
        InputEvent::TimeAdvanced,
    ] {
        assert_eq!(app.handle(2, event), Effects::default());
        assert_eq!(app.session(), paused);
    }

    app.handle(3, InputEvent::Press);
    let completion_at = 3 + app.snapshot(3).remaining_ms;
    app.handle(completion_at, InputEvent::TimeAdvanced);
    let completed = app.session();
    for event in [
        InputEvent::RotateLeft,
        InputEvent::RotateRight,
        InputEvent::LongPress,
        InputEvent::TimeAdvanced,
    ] {
        assert_eq!(app.handle(completion_at, event), Effects::default());
        assert_eq!(app.session(), completed);
    }
}

#[test]
fn deadline_overflow_leaves_state_unchanged() {
    let large = [Preset::new("large", "Large", MAX_SESSION_DURATION_MS)];
    let catalog = Catalog::new(&large, 0).unwrap();
    let (mut app, _) = App::boot(catalog, SettingsLoad::Missing);
    let effects = app.handle(u64::MAX, InputEvent::Press);

    assert_eq!(effects.diagnostic, Some(Diagnostic::DeadlineOverflow));
    assert_eq!(app.session(), SessionState::Idle);
}

#[test]
fn backwards_clock_is_clamped_and_reported() {
    let mut app = boot();
    app.handle(1_000, InputEvent::TimeAdvanced);
    let effects = app.handle(999, InputEvent::RotateRight);

    assert_eq!(effects.diagnostic, Some(Diagnostic::ClockMovedBackwards));
}

#[test]
fn boot_is_idle_and_requests_initial_render() {
    let (app, effects) = App::boot(catalog(), SettingsLoad::Missing);
    assert_eq!(app.session(), SessionState::Idle);
    assert!(effects.render);
    assert_eq!(effects.diagnostic, None);
}

#[test]
fn valid_settings_restore_selected_preset() {
    let record = SettingsRecord {
        schema_version: SETTINGS_SCHEMA_VERSION,
        selected_preset_id: "reading",
    };
    let (index, fallback) = restore_selection(&catalog(), SettingsLoad::Record(record));

    assert_eq!(catalog().preset(index).id.as_str(), "reading");
    assert_eq!(fallback, None);
}

#[test]
fn missing_settings_use_default_without_error() {
    assert_eq!(
        restore_selection(&catalog(), SettingsLoad::Missing),
        (catalog().default_index(), None)
    );
}

#[test]
fn invalid_settings_use_default_and_report_reason() {
    let unsupported = SettingsRecord {
        schema_version: SETTINGS_SCHEMA_VERSION + 1,
        selected_preset_id: "reading",
    };
    let removed = SettingsRecord {
        schema_version: SETTINGS_SCHEMA_VERSION,
        selected_preset_id: "removed-preset",
    };

    assert_eq!(
        restore_selection(&catalog(), SettingsLoad::Corrupt),
        (catalog().default_index(), Some(SettingsFallback::Corrupt))
    );
    assert_eq!(
        restore_selection(&catalog(), SettingsLoad::Record(unsupported)),
        (
            catalog().default_index(),
            Some(SettingsFallback::UnsupportedVersion)
        )
    );
    assert_eq!(
        restore_selection(&catalog(), SettingsLoad::Record(removed)),
        (
            catalog().default_index(),
            Some(SettingsFallback::UnknownPreset)
        )
    );
}

#[test]
fn boot_surfaces_settings_fallback_as_diagnostic() {
    let (app, effects) = App::boot(catalog(), SettingsLoad::Corrupt);
    assert_eq!(
        app.selected_preset(),
        catalog().preset(catalog().default_index())
    );
    assert_eq!(
        effects.diagnostic,
        Some(Diagnostic::SettingsFallback(SettingsFallback::Corrupt))
    );
    assert!(effects.render);
}

#[test]
fn settings_record_cannot_contain_session_state() {
    let record = SettingsRecord {
        schema_version: SETTINGS_SCHEMA_VERSION,
        selected_preset_id: "focus",
    };
    assert_eq!(record.selected_preset_id, "focus");
}
