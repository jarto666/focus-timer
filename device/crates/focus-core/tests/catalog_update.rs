use focus_core::{
    App, Catalog, CatalogConfirmationAction, CatalogError, CatalogStageError,
    CatalogUpdateCoordinator, InputEvent, Preset, SettingsLoad, default_catalog, default_presets,
};

fn combined(custom: &[Preset]) -> Catalog {
    let mut presets = default_presets();
    for preset in custom {
        presets.push(preset.clone()).expect("test catalog must fit");
    }
    Catalog::combined(presets.as_slice()).expect("test catalog must be valid")
}

#[test]
fn combined_catalog_validates_custom_product_contract() {
    let catalog = combined(&[
        Preset::custom("writing", "Writing", 45 * 60_000).unwrap(),
        Preset::custom("planning", "Planning", 10 * 60_000).unwrap(),
    ]);
    assert_eq!(catalog.len(), 7);
    assert!(!catalog.preset(5).built_in);

    let invalid_duration = Preset::custom("bad", "Bad", 60_001).unwrap();
    let mut presets = default_presets();
    presets.push(invalid_duration).unwrap();
    assert_eq!(
        Catalog::combined(presets.as_slice()),
        Err(CatalogError::CustomDurationNotWholeMinute { index: 5 })
    );

    let collision = Preset::custom("focus", "Other Focus", 60_000).unwrap();
    let mut presets = default_presets();
    presets.push(collision).unwrap();
    assert_eq!(
        Catalog::combined(presets.as_slice()),
        Err(CatalogError::DuplicateId {
            first: 1,
            duplicate: 5,
        })
    );
}

#[test]
fn idle_replacement_retains_identifier_or_falls_back_to_focus() {
    let first = combined(&[Preset::custom("writing", "Writing", 45 * 60_000).unwrap()]);
    let (mut app, _) = App::boot(first, SettingsLoad::Missing);
    for _ in 0..3 {
        app.handle(0, InputEvent::RotateRight);
    }
    assert_eq!(app.selected_preset().id.as_str(), "writing");

    let updated = combined(&[Preset::custom("writing", "Writing 2", 60 * 60_000).unwrap()]);
    let effects = app.replace_catalog(updated).unwrap();
    assert_eq!(app.selected_preset().id.as_str(), "writing");
    assert_eq!(app.selected_preset().name.as_str(), "Writing 2");
    assert_eq!(effects.persist_selection, None);

    let effects = app.replace_catalog(default_catalog()).unwrap();
    assert_eq!(app.selected_preset().id.as_str(), "focus");
    assert_eq!(
        effects
            .persist_selection
            .as_ref()
            .map(focus_core::PresetId::as_str),
        Some("focus")
    );
}

#[test]
fn active_session_rejects_catalog_replacement_and_keeps_snapshot() {
    let (mut app, _) = App::boot(default_catalog(), SettingsLoad::Missing);
    let active = app.selected_preset();
    app.handle(0, InputEvent::Press);
    assert!(app.replace_catalog(combined(&[])).is_err());
    assert_eq!(app.snapshot(1).preset, active);
}

#[test]
fn coordinator_covers_confirmation_rejection_expiry_conflict_and_disconnect() {
    let mut coordinator = CatalogUpdateCoordinator::new();
    assert_eq!(
        coordinator.stage(0, false, 3, 3, 1, default_catalog()),
        Err(CatalogStageError::Busy)
    );
    assert_eq!(
        coordinator.stage(0, true, 3, 2, 1, default_catalog()),
        Err(CatalogStageError::Conflict {
            current_revision: 3,
        })
    );

    let staged = coordinator
        .stage(100, true, 3, 3, 7, combined(&[]))
        .unwrap();
    assert_eq!(staged.replaced_proposal_id, None);
    assert_eq!(coordinator.pending_proposal_id(), Some(7));
    assert!(matches!(
        coordinator.handle_input(101, InputEvent::Press),
        CatalogConfirmationAction::Commit(commit) if commit.proposal_id == 7
    ));

    coordinator
        .stage(200, true, 3, 3, 8, combined(&[]))
        .unwrap();
    assert_eq!(
        coordinator.handle_input(201, InputEvent::LongPress),
        CatalogConfirmationAction::Rejected { proposal_id: 8 }
    );

    coordinator
        .stage(300, true, 3, 3, 9, combined(&[]))
        .unwrap();
    assert_eq!(
        coordinator.handle_input(15_300, InputEvent::TimeAdvanced),
        CatalogConfirmationAction::Expired { proposal_id: 9 }
    );

    coordinator
        .stage(400, true, 3, 3, 10, combined(&[]))
        .unwrap();
    assert_eq!(
        coordinator.cancel(),
        CatalogConfirmationAction::Cancelled { proposal_id: 10 }
    );
}

#[test]
fn replacement_proposal_deterministically_cancels_the_previous_one() {
    let mut coordinator = CatalogUpdateCoordinator::new();
    coordinator
        .stage(0, true, 0, 0, 1, default_catalog())
        .unwrap();
    let second = coordinator
        .stage(1, true, 0, 0, 2, default_catalog())
        .unwrap();
    assert_eq!(second.replaced_proposal_id, Some(1));
    assert_eq!(coordinator.pending_proposal_id(), Some(2));
}
