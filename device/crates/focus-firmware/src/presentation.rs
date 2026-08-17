//! Pure snapshot-to-presentation mapping shared by host tests and hardware sinks.

use core::fmt::Write;

use focus_core::{AppSnapshot, ViewState};
use heapless::String;

pub const OLED_WIDTH: u8 = 128;
pub const OLED_HEIGHT: u8 = 64;
pub const RING_PIXELS: usize = 24;
pub const RING_CHANNEL_LIMIT: u8 = 32;

pub type TimeText = String<24>;

pub const TFT_PROGRESS_MAX: u16 = 1_000;
pub const TFT_ACTIVE_FRAME_INTERVAL_MS: u32 = 170;
pub const TFT_QUIESCENT_FRAME_INTERVAL_MS: u32 = 750;
pub const TFT_STATIC_RETRY_INTERVAL_MS: u32 = 1_000;
pub const TFT_STATIC_FAILURE_THRESHOLD: u8 = 3;

/// Semantic screen selected by the authoritative runtime. Rendering code may
/// style these variants differently, but it cannot infer or mutate app state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TftScene {
    Session(ViewState),
    CatalogConfirmation,
}

/// Bounded immutable input for one TFT render generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TftView {
    pub generation: u64,
    pub scene: TftScene,
    pub state_label: &'static str,
    pub preset_name: String<32>,
    pub time: TimeText,
    /// Elapsed fraction in permille, clamped to [`TFT_PROGRESS_MAX`].
    pub progress: u16,
    pub hint: &'static str,
}

/// Whether the disposable atmospheric layer may advance for a frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TftBackgroundMode {
    Animated,
    Static,
}

/// Pure scheduling decision consumed by the display worker.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TftRenderPolicy {
    pub background: TftBackgroundMode,
    pub render_immediately: bool,
    pub next_frame_after_ms: u32,
}

/// Health and urgency signals owned by the display boundary, not the app.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TftRenderSignals {
    pub recent_interaction: bool,
    pub frame_delayed: bool,
    pub consecutive_failures: u8,
}

#[must_use]
pub fn tft_view(snapshot: &AppSnapshot, generation: u64) -> TftView {
    let oled = oled_view(snapshot.clone());
    TftView {
        generation,
        scene: TftScene::Session(snapshot.state),
        state_label: oled.state_label,
        preset_name: oled.preset_name,
        time: oled.time,
        progress: progress_permille(snapshot),
        hint: oled.hint,
    }
}

#[must_use]
/// Builds the physical approval screen without taking ownership of the pending
/// catalog or of the encoder decision.
pub fn tft_catalog_confirmation_view(custom_count: usize, generation: u64) -> TftView {
    let oled = catalog_confirmation_view(custom_count);
    TftView {
        generation,
        scene: TftScene::CatalogConfirmation,
        state_label: oled.state_label,
        preset_name: oled.preset_name,
        time: oled.time,
        progress: 0,
        hint: oled.hint,
    }
}

#[must_use]
pub const fn tft_render_policy(scene: TftScene, signals: TftRenderSignals) -> TftRenderPolicy {
    let static_fallback =
        signals.frame_delayed || signals.consecutive_failures >= TFT_STATIC_FAILURE_THRESHOLD;
    let background = if static_fallback {
        TftBackgroundMode::Static
    } else {
        TftBackgroundMode::Animated
    };
    let render_immediately = signals.recent_interaction;
    let next_frame_after_ms = if static_fallback {
        TFT_STATIC_RETRY_INTERVAL_MS
    } else if signals.recent_interaction || matches!(scene, TftScene::Session(ViewState::Running)) {
        TFT_ACTIVE_FRAME_INTERVAL_MS
    } else {
        TFT_QUIESCENT_FRAME_INTERVAL_MS
    };

    TftRenderPolicy {
        background,
        render_immediately,
        next_frame_after_ms,
    }
}

/// Fixed 128x64 text bands consumed by the SSD1306 renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OledLayout {
    pub state_y: u8,
    pub preset_y: u8,
    pub time_y: u8,
    pub hint_y: u8,
}

pub const OLED_LAYOUT: OledLayout = OledLayout {
    state_y: 0,
    preset_y: 13,
    time_y: 28,
    hint_y: 54,
};

/// Text model for one complete OLED frame. It contains no display handle and
/// cannot mutate application state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OledView {
    pub state_label: &'static str,
    pub preset_name: String<32>,
    pub time: TimeText,
    pub hint: &'static str,
}

#[must_use]
pub fn oled_view(snapshot: AppSnapshot) -> OledView {
    let (state_label, hint) = match snapshot.state {
        ViewState::Idle => ("READY", "Press to start"),
        ViewState::Running => ("FOCUS", "Press to pause"),
        ViewState::Paused => ("PAUSED", "Hold to cancel"),
        ViewState::Completed => ("COMPLETE", "Press to dismiss"),
    };

    OledView {
        state_label,
        preset_name: snapshot.preset.name,
        time: format_duration(snapshot.remaining_ms),
        hint,
    }
}

#[must_use]
/// Builds the dedicated physical-approval frame.
///
/// # Panics
///
/// Panics only if the fixed product copy no longer fits its declared OLED buffers.
pub fn catalog_confirmation_view(custom_count: usize) -> OledView {
    let mut preset_name = String::new();
    write!(preset_name, "{custom_count} custom presets")
        .expect("bounded custom count fits the OLED line");
    let mut time = TimeText::new();
    time.push_str("UPDATE?")
        .expect("confirmation label fits the OLED time buffer");
    OledView {
        state_label: "PHONE REQUEST",
        preset_name,
        time,
        hint: "Press yes / hold no",
    }
}

/// Low-current RGB value. Every channel is capped before a frame reaches a
/// hardware sink.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Rgb {
    pub const OFF: Self = Self {
        red: 0,
        green: 0,
        blue: 0,
    };
    pub const RUNNING: Self = Self {
        red: 0,
        green: 10,
        blue: RING_CHANNEL_LIMIT,
    };
    pub const PAUSED: Self = Self {
        red: RING_CHANNEL_LIMIT,
        green: 8,
        blue: 0,
    };
    pub const COMPLETED: Self = Self {
        red: 0,
        green: RING_CHANNEL_LIMIT,
        blue: 4,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RingFrame {
    pub pixels: [Rgb; RING_PIXELS],
}

impl Default for RingFrame {
    fn default() -> Self {
        Self {
            pixels: [Rgb::OFF; RING_PIXELS],
        }
    }
}

/// Derives a static, non-animated frame from the immutable app snapshot.
#[must_use]
#[allow(clippy::needless_pass_by_value)]
pub fn ring_frame(snapshot: AppSnapshot) -> RingFrame {
    let mut frame = RingFrame::default();
    match snapshot.state {
        ViewState::Idle => {}
        ViewState::Running => {
            let elapsed = snapshot
                .preset
                .duration_ms
                .saturating_sub(snapshot.remaining_ms);
            let lit = pixel_progress(elapsed, snapshot.preset.duration_ms);
            frame.pixels[..lit].fill(Rgb::RUNNING);
        }
        ViewState::Paused => frame.pixels.fill(Rgb::PAUSED),
        ViewState::Completed => frame.pixels.fill(Rgb::COMPLETED),
    }
    frame
}

/// Boundary for a real WS2812 adapter or a failing/disconnected test double.
pub trait RingSink {
    type Error;

    /// Writes one complete frame.
    ///
    /// # Errors
    ///
    /// Returns the hardware adapter's error when the ring is disconnected or
    /// the data transfer fails.
    fn write(&mut self, frame: &RingFrame) -> Result<(), Self::Error>;
}

/// Attempts one frame write. The caller may log the error, while the app state
/// remains outside this adapter and therefore cannot be rolled back.
///
/// # Errors
///
/// Returns the error reported by [`RingSink::write`].
pub fn render_ring<S: RingSink>(sink: &mut S, snapshot: AppSnapshot) -> Result<(), S::Error> {
    sink.write(&ring_frame(snapshot))
}

fn format_duration(remaining_ms: u64) -> TimeText {
    let total_seconds = remaining_ms / 1_000 + u64::from(remaining_ms % 1_000 != 0);
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let mut text = TimeText::new();
    write!(text, "{minutes:02}:{seconds:02}")
        .expect("validated session duration must fit the OLED time buffer");
    text
}

fn progress_permille(snapshot: &AppSnapshot) -> u16 {
    if snapshot.state == ViewState::Completed {
        return TFT_PROGRESS_MAX;
    }
    if snapshot.state == ViewState::Idle || snapshot.preset.duration_ms == 0 {
        return 0;
    }
    let elapsed_ms = snapshot
        .preset
        .duration_ms
        .saturating_sub(snapshot.remaining_ms);
    let progress = elapsed_ms
        .saturating_mul(u64::from(TFT_PROGRESS_MAX))
        .checked_div(snapshot.preset.duration_ms)
        .unwrap_or(0)
        .min(u64::from(TFT_PROGRESS_MAX));
    u16::try_from(progress).unwrap_or(TFT_PROGRESS_MAX)
}

fn pixel_progress(elapsed_ms: u64, duration_ms: u64) -> usize {
    if duration_ms == 0 {
        return 0;
    }
    let pixels = elapsed_ms
        .saturating_mul(RING_PIXELS as u64)
        .checked_div(duration_ms)
        .unwrap_or(0)
        .min(RING_PIXELS as u64);
    usize::try_from(pixels).unwrap_or(RING_PIXELS)
}
