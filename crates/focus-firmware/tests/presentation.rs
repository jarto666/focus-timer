use focus_core::{App, AppSnapshot, Catalog, DEFAULT_PRESETS, InputEvent, SettingsLoad, ViewState};
use focus_firmware::presentation::{
    OLED_HEIGHT, OLED_LAYOUT, OLED_WIDTH, RING_CHANNEL_LIMIT, RING_PIXELS, Rgb, RingFrame,
    RingSink, oled_view, render_ring, ring_frame,
};

fn boot() -> App {
    App::boot(
        Catalog::new(&DEFAULT_PRESETS, 2).unwrap(),
        SettingsLoad::Missing,
    )
    .0
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
fn running_ring_shows_bounded_progress() {
    let preset = DEFAULT_PRESETS[2];
    let frame = ring_frame(AppSnapshot {
        state: ViewState::Running,
        preset,
        remaining_ms: preset.duration_ms / 2,
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
    let preset = DEFAULT_PRESETS[2];
    let snapshot = |state| AppSnapshot {
        state,
        preset,
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
        render_ring(&mut Disconnected, initial),
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
