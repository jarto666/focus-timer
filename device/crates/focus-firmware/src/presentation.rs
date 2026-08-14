//! Pure snapshot-to-presentation mapping shared by host tests and hardware sinks.

use core::fmt::Write;

use focus_core::{AppSnapshot, ViewState};
use heapless::String;

pub const OLED_WIDTH: u8 = 128;
pub const OLED_HEIGHT: u8 = 64;
pub const RING_PIXELS: usize = 24;
pub const RING_CHANNEL_LIMIT: u8 = 32;

pub type TimeText = String<24>;

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
