//! Polling-based EC11 quadrature and push-switch classification.

use focus_core::InputEvent;

/// Initial switch debounce threshold to validate on the physical encoder.
pub const SWITCH_DEBOUNCE_MS: u64 = 25;
/// Initial boundary between a short and long press.
pub const LONG_PRESS_MS: u64 = 800;

const QUARTER_STEPS_PER_DETENT: i8 = 4;

/// Semantic events that can be produced by one GPIO sample.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SampleEvents {
    pub rotation: Option<InputEvent>,
    pub button: Option<InputEvent>,
}

/// Stateful EC11 decoder. The initial A/B state is treated as the stable
/// detent; a direction is emitted only after a full Gray-code cycle returns to
/// that state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuadratureDecoder {
    detent: u8,
    previous: u8,
    quarter_steps: i8,
}

impl QuadratureDecoder {
    #[must_use]
    pub const fn new(a_high: bool, b_high: bool) -> Self {
        let state = pin_state(a_high, b_high);
        Self {
            detent: state,
            previous: state,
            quarter_steps: 0,
        }
    }

    /// Samples the two encoder phases. Same-state samples are ignored and
    /// two-bit jumps discard the incomplete gesture as electrically ambiguous.
    pub fn sample(&mut self, a_high: bool, b_high: bool) -> Option<InputEvent> {
        let current = pin_state(a_high, b_high);
        if current == self.previous {
            return None;
        }

        let transition = (self.previous << 2) | current;
        self.previous = current;
        let delta = match transition {
            0b0001 | 0b0111 | 0b1110 | 0b1000 => 1,
            0b0010 | 0b1011 | 0b1101 | 0b0100 => -1,
            _ => {
                self.quarter_steps = 0;
                return None;
            }
        };
        self.quarter_steps = self.quarter_steps.saturating_add(delta);

        if current != self.detent {
            return None;
        }

        let event = match self.quarter_steps {
            QUARTER_STEPS_PER_DETENT => Some(InputEvent::RotateRight),
            -4 => Some(InputEvent::RotateLeft),
            _ => None,
        };
        self.quarter_steps = 0;
        event
    }
}

/// Debounces the active-low EC11 switch after the GPIO adapter has converted
/// its level to `pressed: bool`. A long gesture is emitted at most once while
/// held; its later release cannot also emit a short press.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SwitchClassifier {
    stable_pressed: bool,
    candidate: Option<(bool, u64)>,
    press_started_ms: Option<u64>,
    long_press_emitted: bool,
    armed: bool,
}

impl SwitchClassifier {
    /// Creates a classifier from the boot-time raw state. A switch already
    /// held during boot is ignored until it is released and pressed again.
    #[must_use]
    pub const fn new(initial_pressed: bool) -> Self {
        Self {
            stable_pressed: initial_pressed,
            candidate: None,
            press_started_ms: None,
            long_press_emitted: false,
            armed: !initial_pressed,
        }
    }

    pub fn sample(&mut self, now_ms: u64, raw_pressed: bool) -> Option<InputEvent> {
        if raw_pressed == self.stable_pressed {
            self.candidate = None;
        } else {
            let candidate_since = match self.candidate {
                Some((candidate, since)) if candidate == raw_pressed => since,
                _ => {
                    self.candidate = Some((raw_pressed, now_ms));
                    now_ms
                }
            };

            if now_ms.saturating_sub(candidate_since) >= SWITCH_DEBOUNCE_MS {
                self.stable_pressed = raw_pressed;
                self.candidate = None;

                if raw_pressed {
                    if self.armed {
                        self.press_started_ms = Some(candidate_since);
                        self.long_press_emitted = false;
                    }
                } else {
                    self.armed = true;
                    return self.classify_release(candidate_since);
                }
            }
        }

        self.classify_held(now_ms, raw_pressed)
    }

    fn classify_held(&mut self, now_ms: u64, raw_pressed: bool) -> Option<InputEvent> {
        let started = self.press_started_ms?;
        if self.stable_pressed
            && raw_pressed
            && !self.long_press_emitted
            && now_ms.saturating_sub(started) >= LONG_PRESS_MS
        {
            self.long_press_emitted = true;
            return Some(InputEvent::LongPress);
        }
        None
    }

    fn classify_release(&mut self, released_ms: u64) -> Option<InputEvent> {
        let started = self.press_started_ms.take()?;
        if self.long_press_emitted {
            return None;
        }

        if released_ms.saturating_sub(started) >= LONG_PRESS_MS {
            self.long_press_emitted = true;
            Some(InputEvent::LongPress)
        } else {
            Some(InputEvent::Press)
        }
    }
}

/// Combines both polling classifiers without coupling them to GPIO ownership.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EncoderInput {
    rotation: QuadratureDecoder,
    button: SwitchClassifier,
}

impl EncoderInput {
    #[must_use]
    pub const fn new(a_high: bool, b_high: bool, pressed: bool) -> Self {
        Self {
            rotation: QuadratureDecoder::new(a_high, b_high),
            button: SwitchClassifier::new(pressed),
        }
    }

    pub fn sample(
        &mut self,
        now_ms: u64,
        a_high: bool,
        b_high: bool,
        pressed: bool,
    ) -> SampleEvents {
        SampleEvents {
            rotation: self.rotation.sample(a_high, b_high),
            button: self.button.sample(now_ms, pressed),
        }
    }
}

const fn pin_state(a_high: bool, b_high: bool) -> u8 {
    ((a_high as u8) << 1) | (b_high as u8)
}
