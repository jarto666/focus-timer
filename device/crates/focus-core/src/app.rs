use core::cmp;

use crate::{Catalog, Preset, PresetId, SettingsFallback, SettingsLoad, restore_selection};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputEvent {
    RotateLeft,
    RotateRight,
    Press,
    LongPress,
    TimeAdvanced,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeedbackPattern {
    Start,
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Diagnostic {
    SettingsFallback(SettingsFallback),
    DeadlineOverflow,
    ClockMovedBackwards,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionOutcomeKind {
    Completed,
    Cancelled,
}

/// Semantic result of one committed session lifecycle.
///
/// Adapters may persist this value after the core transition has completed,
/// but persistence failure cannot roll the transition back.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionOutcome {
    pub kind: SessionOutcomeKind,
    pub preset: Preset,
    pub planned_duration_ms: u64,
    pub active_duration_ms: u64,
}

/// Best-effort work for firmware adapters after state has been committed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Effects {
    pub render: bool,
    pub feedback: Option<FeedbackPattern>,
    pub persist_selection: Option<PresetId>,
    pub diagnostic: Option<Diagnostic>,
    pub outcome: Option<SessionOutcome>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionState {
    Idle,
    Running { active: Preset, deadline_ms: u64 },
    Paused { active: Preset, remaining_ms: u64 },
    Completed { active: Preset },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewState {
    Idle,
    Running,
    Paused,
    Completed,
}

/// Immutable data consumed by display and feedback adapters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppSnapshot {
    pub state: ViewState,
    pub preset: Preset,
    pub remaining_ms: u64,
}

/// Single owner of preset selection and session state.
#[derive(Debug)]
pub struct App {
    catalog: Catalog,
    selected_index: usize,
    session: SessionState,
    last_now_ms: u64,
}

impl App {
    /// Boots safely into Idle and reports invalid persisted settings as an effect.
    #[must_use]
    pub fn boot(catalog: Catalog, settings: SettingsLoad<'_>) -> (Self, Effects) {
        let (selected_index, fallback) = restore_selection(catalog, settings);
        let effects = Effects {
            render: true,
            diagnostic: fallback.map(Diagnostic::SettingsFallback),
            ..Effects::default()
        };
        (
            Self {
                catalog,
                selected_index,
                session: SessionState::Idle,
                last_now_ms: 0,
            },
            effects,
        )
    }

    #[must_use]
    pub const fn session(&self) -> SessionState {
        self.session
    }

    #[must_use]
    pub fn selected_preset(&self) -> Preset {
        self.catalog.preset(self.selected_index)
    }

    /// Applies one semantic input at a monotonic timestamp.
    pub fn handle(&mut self, now_ms: u64, event: InputEvent) -> Effects {
        let (now_ms, clock_diagnostic) = if now_ms < self.last_now_ms {
            (self.last_now_ms, Some(Diagnostic::ClockMovedBackwards))
        } else {
            self.last_now_ms = now_ms;
            (now_ms, None)
        };

        if self.complete_if_due(now_ms) {
            let SessionState::Completed { active } = self.session else {
                unreachable!();
            };
            return Effects {
                render: true,
                feedback: Some(FeedbackPattern::Complete),
                diagnostic: clock_diagnostic,
                outcome: Some(SessionOutcome {
                    kind: SessionOutcomeKind::Completed,
                    preset: active,
                    planned_duration_ms: active.duration_ms,
                    active_duration_ms: active.duration_ms,
                }),
                ..Effects::default()
            };
        }

        let mut effects = match (self.session, event) {
            (SessionState::Idle, InputEvent::RotateLeft) => self.rotate_left(),
            (SessionState::Idle, InputEvent::RotateRight) => self.rotate_right(),
            (SessionState::Idle, InputEvent::Press) => self.start(now_ms),
            (
                SessionState::Running {
                    active,
                    deadline_ms,
                },
                InputEvent::Press,
            ) => {
                self.session = SessionState::Paused {
                    active,
                    remaining_ms: deadline_ms.saturating_sub(now_ms),
                };
                render_effect()
            }
            (
                SessionState::Paused {
                    active,
                    remaining_ms,
                },
                InputEvent::Press,
            ) => self.resume(now_ms, active, remaining_ms),
            (
                SessionState::Running {
                    active,
                    deadline_ms,
                },
                InputEvent::LongPress,
            ) => {
                let remaining_ms = deadline_ms.saturating_sub(now_ms);
                self.session = SessionState::Idle;
                cancellation_effect(active, remaining_ms)
            }
            (
                SessionState::Paused {
                    active,
                    remaining_ms,
                },
                InputEvent::LongPress,
            ) => {
                self.session = SessionState::Idle;
                cancellation_effect(active, remaining_ms)
            }
            (SessionState::Completed { .. }, InputEvent::Press) => {
                self.session = SessionState::Idle;
                render_effect()
            }
            _ => Effects::default(),
        };

        if effects.diagnostic.is_none() {
            effects.diagnostic = clock_diagnostic;
        }
        effects
    }

    /// Builds a read-only view; it never transitions or owns elapsed-time truth.
    #[must_use]
    pub fn snapshot(&self, now_ms: u64) -> AppSnapshot {
        match self.session {
            SessionState::Idle => {
                let preset = self.selected_preset();
                AppSnapshot {
                    state: ViewState::Idle,
                    preset,
                    remaining_ms: preset.duration_ms,
                }
            }
            SessionState::Running {
                active,
                deadline_ms,
            } => AppSnapshot {
                state: ViewState::Running,
                preset: active,
                remaining_ms: deadline_ms.saturating_sub(cmp::max(now_ms, self.last_now_ms)),
            },
            SessionState::Paused {
                active,
                remaining_ms,
            } => AppSnapshot {
                state: ViewState::Paused,
                preset: active,
                remaining_ms,
            },
            SessionState::Completed { active } => AppSnapshot {
                state: ViewState::Completed,
                preset: active,
                remaining_ms: 0,
            },
        }
    }

    fn complete_if_due(&mut self, now_ms: u64) -> bool {
        if let SessionState::Running {
            active,
            deadline_ms,
        } = self.session
            && now_ms >= deadline_ms
        {
            self.session = SessionState::Completed { active };
            return true;
        }
        false
    }

    fn rotate_left(&mut self) -> Effects {
        self.selected_index = if self.selected_index == 0 {
            self.catalog.len() - 1
        } else {
            self.selected_index - 1
        };
        selection_effect(self.selected_preset().id)
    }

    fn rotate_right(&mut self) -> Effects {
        self.selected_index = (self.selected_index + 1) % self.catalog.len();
        selection_effect(self.selected_preset().id)
    }

    fn start(&mut self, now_ms: u64) -> Effects {
        let active = self.selected_preset();
        let Some(deadline_ms) = now_ms.checked_add(active.duration_ms) else {
            return diagnostic_effect(Diagnostic::DeadlineOverflow);
        };
        self.session = SessionState::Running {
            active,
            deadline_ms,
        };
        Effects {
            render: true,
            feedback: Some(FeedbackPattern::Start),
            ..Effects::default()
        }
    }

    fn resume(&mut self, now_ms: u64, active: Preset, remaining_ms: u64) -> Effects {
        let Some(deadline_ms) = now_ms.checked_add(remaining_ms) else {
            return diagnostic_effect(Diagnostic::DeadlineOverflow);
        };
        self.session = SessionState::Running {
            active,
            deadline_ms,
        };
        render_effect()
    }
}

fn render_effect() -> Effects {
    Effects {
        render: true,
        ..Effects::default()
    }
}

fn selection_effect(id: PresetId) -> Effects {
    Effects {
        render: true,
        persist_selection: Some(id),
        ..Effects::default()
    }
}

fn diagnostic_effect(diagnostic: Diagnostic) -> Effects {
    Effects {
        diagnostic: Some(diagnostic),
        ..Effects::default()
    }
}

fn cancellation_effect(active: Preset, remaining_ms: u64) -> Effects {
    Effects {
        render: true,
        outcome: Some(SessionOutcome {
            kind: SessionOutcomeKind::Cancelled,
            preset: active,
            planned_duration_ms: active.duration_ms,
            active_duration_ms: active.duration_ms.saturating_sub(remaining_ms),
        }),
        ..Effects::default()
    }
}
