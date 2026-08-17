use focus_core::{App, AppSnapshot, InputEvent, SettingsLoad, ViewState, default_catalog};
use focus_firmware::presentation::{
    OLED_HEIGHT, OLED_LAYOUT, OLED_WIDTH, RING_CHANNEL_LIMIT, RING_PIXELS, Rgb, RingFrame,
    RingSink, TFT_ACTIVE_FRAME_INTERVAL_MS, TFT_PROGRESS_MAX, TFT_QUIESCENT_FRAME_INTERVAL_MS,
    TFT_STATIC_FAILURE_THRESHOLD, TFT_STATIC_RETRY_INTERVAL_MS, TftBackgroundMode,
    TftRenderSignals, TftScene, catalog_confirmation_view, oled_view, render_ring, ring_frame,
    tft_catalog_confirmation_view, tft_render_policy, tft_view,
};

fn boot() -> App {
    App::boot(default_catalog(), SettingsLoad::Missing).0
}

#[test]
fn oled_layout_fits_the_purchased_display() {
    assert_eq!((OLED_WIDTH, OLED_HEIGHT), (128, 64));
    let layout = std::hint::black_box(OLED_LAYOUT);
    assert!(layout.state_y < layout.preset_y);
    assert!(layout.preset_y < layout.time_y);
    assert!(layout.time_y < layout.hint_y);
    assert!(layout.hint_y < OLED_HEIGHT);
}

#[test]
fn oled_models_make_all_states_unambiguous() {
    let mut app = boot();
    let idle = oled_view(app.snapshot(0));
    assert_eq!(idle.state_label, "READY");
    assert_eq!(idle.preset_name, "Pomodoro");
    assert_eq!(idle.time.as_str(), "25:00");
    assert_eq!(idle.hint, "Press to start");

    app.handle(0, InputEvent::Press);
    let running = oled_view(app.snapshot(1_000));
    assert_eq!(running.state_label, "FOCUS");
    assert_eq!(running.time.as_str(), "24:59");

    app.handle(1_000, InputEvent::Press);
    let paused = oled_view(app.snapshot(100_000));
    assert_eq!(paused.state_label, "PAUSED");
    assert_eq!(paused.time, running.time);
    assert_eq!(paused.hint, "Hold to cancel");

    app.handle(100_000, InputEvent::Press);
    let completion_at = 100_000 + paused_time_ms(&paused);
    app.handle(completion_at, InputEvent::TimeAdvanced);
    let completed = oled_view(app.snapshot(completion_at));
    assert_eq!(completed.state_label, "COMPLETE");
    assert_eq!(completed.time.as_str(), "00:00");
    assert_eq!(completed.hint, "Press to dismiss");
}

#[test]
fn catalog_confirmation_is_unambiguous_and_bounded() {
    let view = catalog_confirmation_view(8);
    assert_eq!(view.state_label, "PHONE REQUEST");
    assert_eq!(view.preset_name.as_str(), "8 custom presets");
    assert_eq!(view.time.as_str(), "UPDATE?");
    assert!(view.hint.contains("Press"));
}

#[test]
fn tft_models_make_all_states_unambiguous_and_keep_generation() {
    let mut app = boot();
    let idle = tft_view(&app.snapshot(0), 41);
    assert_eq!(idle.generation, 41);
    assert_eq!(idle.scene, TftScene::Session(ViewState::Idle));
    assert_eq!(idle.state_label, "READY");
    assert_eq!(idle.preset_name, "Pomodoro");
    assert_eq!(idle.time.as_str(), "25:00");
    assert_eq!(idle.progress, 0);

    app.handle(0, InputEvent::Press);
    let running = tft_view(&app.snapshot(750_000), 42);
    assert_eq!(running.scene, TftScene::Session(ViewState::Running));
    assert_eq!(running.state_label, "FOCUS");
    assert_eq!(running.time.as_str(), "12:30");
    assert_eq!(running.progress, 500);

    app.handle(750_000, InputEvent::Press);
    let paused = tft_view(&app.snapshot(999_000), 43);
    assert_eq!(paused.scene, TftScene::Session(ViewState::Paused));
    assert_eq!(paused.state_label, "PAUSED");
    assert_eq!(paused.time, running.time);
    assert_eq!(paused.progress, running.progress);

    app.handle(999_000, InputEvent::Press);
    app.handle(1_749_000, InputEvent::TimeAdvanced);
    let completed = tft_view(&app.snapshot(1_749_000), 44);
    assert_eq!(completed.scene, TftScene::Session(ViewState::Completed));
    assert_eq!(completed.state_label, "COMPLETE");
    assert_eq!(completed.time.as_str(), "00:00");
    assert_eq!(completed.progress, TFT_PROGRESS_MAX);
}

#[test]
fn tft_catalog_confirmation_is_bounded_and_semantic() {
    let view = tft_catalog_confirmation_view(8, 19);
    assert_eq!(view.generation, 19);
    assert_eq!(view.scene, TftScene::CatalogConfirmation);
    assert_eq!(view.state_label, "PHONE REQUEST");
    assert_eq!(view.preset_name.as_str(), "8 custom presets");
    assert_eq!(view.time.as_str(), "UPDATE?");
    assert_eq!(view.progress, 0);
    assert!(view.hint.contains("Press"));
}

#[test]
fn tft_progress_is_zero_full_and_clamped() {
    let preset = default_catalog().preset(2);
    let make = |state, remaining_ms| {
        tft_view(
            &AppSnapshot {
                state,
                preset: preset.clone(),
                remaining_ms,
            },
            1,
        )
    };

    assert_eq!(make(ViewState::Idle, preset.duration_ms).progress, 0);
    assert_eq!(make(ViewState::Running, preset.duration_ms + 1).progress, 0);
    assert_eq!(make(ViewState::Running, 0).progress, TFT_PROGRESS_MAX);
    assert_eq!(
        make(ViewState::Completed, u64::MAX).progress,
        TFT_PROGRESS_MAX
    );
}

#[test]
fn tft_keeps_maximum_length_preset_names() {
    let preset =
        focus_core::Preset::custom("custom", "12345678901234567890123456789012", 60_000).unwrap();
    let view = tft_view(
        &AppSnapshot {
            state: ViewState::Idle,
            preset,
            remaining_ms: 60_000,
        },
        u64::MAX,
    );
    assert_eq!(view.preset_name.len(), 32);
    assert_eq!(view.generation, u64::MAX);
}

#[test]
fn tft_time_changes_only_at_visible_second_boundaries() {
    let app = boot();
    assert_eq!(tft_view(&app.snapshot(0), 1).time.as_str(), "25:00");

    let mut app = app;
    app.handle(0, InputEvent::Press);
    assert_eq!(tft_view(&app.snapshot(1), 2).time.as_str(), "25:00");
    assert_eq!(tft_view(&app.snapshot(999), 3).time.as_str(), "25:00");
    assert_eq!(tft_view(&app.snapshot(1_000), 4).time.as_str(), "24:59");
}

#[test]
fn tft_cadence_prioritizes_interaction_and_degrades_to_static() {
    let immediate = tft_render_policy(
        TftScene::Session(ViewState::Idle),
        TftRenderSignals {
            recent_interaction: true,
            ..TftRenderSignals::default()
        },
    );
    assert!(immediate.render_immediately);
    assert_eq!(immediate.background, TftBackgroundMode::Animated);
    assert_eq!(immediate.next_frame_after_ms, TFT_ACTIVE_FRAME_INTERVAL_MS);

    let running = tft_render_policy(
        TftScene::Session(ViewState::Running),
        TftRenderSignals::default(),
    );
    assert!(!running.render_immediately);
    assert_eq!(running.next_frame_after_ms, TFT_ACTIVE_FRAME_INTERVAL_MS);

    let quiescent = tft_render_policy(
        TftScene::Session(ViewState::Paused),
        TftRenderSignals::default(),
    );
    assert_eq!(
        quiescent.next_frame_after_ms,
        TFT_QUIESCENT_FRAME_INTERVAL_MS
    );

    for signals in [
        TftRenderSignals {
            frame_delayed: true,
            ..TftRenderSignals::default()
        },
        TftRenderSignals {
            consecutive_failures: TFT_STATIC_FAILURE_THRESHOLD,
            ..TftRenderSignals::default()
        },
    ] {
        let degraded = tft_render_policy(TftScene::CatalogConfirmation, signals);
        assert_eq!(degraded.background, TftBackgroundMode::Static);
        assert_eq!(degraded.next_frame_after_ms, TFT_STATIC_RETRY_INTERVAL_MS);
    }
}

#[test]
fn running_ring_shows_bounded_progress() {
    let preset = default_catalog().preset(2);
    let remaining_ms = preset.duration_ms / 2;
    let frame = ring_frame(AppSnapshot {
        state: ViewState::Running,
        preset,
        remaining_ms,
    });

    assert_eq!(
        frame
            .pixels
            .iter()
            .filter(|&&pixel| pixel == Rgb::RUNNING)
            .count(),
        RING_PIXELS / 2
    );
    assert!(frame.pixels.iter().all(|pixel| {
        pixel.red <= RING_CHANNEL_LIMIT
            && pixel.green <= RING_CHANNEL_LIMIT
            && pixel.blue <= RING_CHANNEL_LIMIT
    }));
}

#[test]
fn idle_paused_and_completed_ring_frames_are_distinct() {
    let preset = default_catalog().preset(2);
    let snapshot = |state| AppSnapshot {
        state,
        preset: preset.clone(),
        remaining_ms: preset.duration_ms,
    };

    assert_eq!(ring_frame(snapshot(ViewState::Idle)), RingFrame::default());
    assert!(
        ring_frame(snapshot(ViewState::Paused))
            .pixels
            .iter()
            .all(|&pixel| pixel == Rgb::PAUSED)
    );
    assert!(
        ring_frame(snapshot(ViewState::Completed))
            .pixels
            .iter()
            .all(|&pixel| pixel == Rgb::COMPLETED)
    );
}

#[derive(Debug, Eq, PartialEq)]
struct Disconnected;

impl RingSink for Disconnected {
    type Error = &'static str;

    fn write(&mut self, _frame: &RingFrame) -> Result<(), Self::Error> {
        Err("ring disconnected")
    }
}

#[test]
fn ring_failure_cannot_stop_timer_or_oled_mapping() {
    let mut app = boot();
    app.handle(0, InputEvent::Press);
    let initial = app.snapshot(0);
    assert_eq!(
        render_ring(&mut Disconnected, initial.clone()),
        Err("ring disconnected")
    );

    app.handle(1_000, InputEvent::TimeAdvanced);
    let later = app.snapshot(1_000);
    assert_eq!(later.state, ViewState::Running);
    assert_eq!(later.remaining_ms, initial.remaining_ms - 1_000);
    assert_eq!(oled_view(later).time.as_str(), "24:59");
}

fn paused_time_ms(view: &focus_firmware::presentation::OledView) -> u64 {
    let (minutes, seconds) = view.time.split_once(':').unwrap();
    minutes.parse::<u64>().unwrap() * 60_000 + seconds.parse::<u64>().unwrap() * 1_000
}
