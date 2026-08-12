//! Non-blocking cadence model for an active on/off buzzer.

use focus_core::FeedbackPattern;

#[derive(Clone, Copy)]
struct Step {
    on: bool,
    duration_ms: u64,
}

const START_STEPS: [Step; 1] = [Step {
    on: true,
    duration_ms: 80,
}];

const COMPLETE_STEPS: [Step; 5] = [
    Step {
        on: true,
        duration_ms: 120,
    },
    Step {
        on: false,
        duration_ms: 100,
    },
    Step {
        on: true,
        duration_ms: 120,
    },
    Step {
        on: false,
        duration_ms: 100,
    },
    Step {
        on: true,
        duration_ms: 280,
    },
];

/// Advances short active-buzzer patterns from monotonic timestamps.
///
/// The caller owns the GPIO and applies only the level changes returned by
/// [`Self::update`]. No sleep, GPIO access, or application state lives here.
#[derive(Debug, Default)]
pub struct BuzzerCadence {
    pattern: Option<FeedbackPattern>,
    step_index: usize,
    deadline_ms: u64,
    output_on: bool,
}

impl BuzzerCadence {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pattern: None,
            step_index: 0,
            deadline_ms: 0,
            output_on: false,
        }
    }

    /// Starts or replaces a cadence and returns the initial GPIO level.
    #[must_use]
    pub fn trigger(&mut self, now_ms: u64, pattern: FeedbackPattern) -> bool {
        let first = steps(pattern)[0];
        self.pattern = Some(pattern);
        self.step_index = 0;
        self.deadline_ms = now_ms.saturating_add(first.duration_ms);
        self.output_on = first.on;
        self.output_on
    }

    /// Returns `Some(level)` only when the physical output should change.
    pub fn update(&mut self, now_ms: u64) -> Option<bool> {
        let mut changed = None;

        while let Some(pattern) = self.pattern {
            if now_ms < self.deadline_ms {
                break;
            }

            let pattern_steps = steps(pattern);
            self.step_index += 1;
            if self.step_index == pattern_steps.len() {
                self.pattern = None;
                if self.output_on {
                    self.output_on = false;
                    changed = Some(false);
                }
                break;
            }

            let next = pattern_steps[self.step_index];
            self.deadline_ms = self.deadline_ms.saturating_add(next.duration_ms);
            if self.output_on != next.on {
                self.output_on = next.on;
                changed = Some(next.on);
            }
        }

        changed
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.pattern.is_some()
    }
}

fn steps(pattern: FeedbackPattern) -> &'static [Step] {
    match pattern {
        FeedbackPattern::Start => &START_STEPS,
        FeedbackPattern::Complete => &COMPLETE_STEPS,
    }
}
