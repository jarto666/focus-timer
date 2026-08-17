//! Non-blocking latest-value boundary between app and display work.

extern crate std;

use std::sync::{Arc, Mutex, TryLockError};

use crate::presentation::TftView;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublishResult {
    Accepted,
    Stale,
    Busy,
}

#[derive(Clone)]
pub struct LatestViewSlot {
    inner: Arc<Mutex<SlotState>>,
}

struct SlotState {
    latest_generation: u64,
    pending: Option<TftView>,
}

impl LatestViewSlot {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SlotState {
                latest_generation: 0,
                pending: None,
            })),
        }
    }

    /// Attempts to replace the pending request without ever waiting for the worker.
    #[must_use]
    pub fn try_publish(&self, view: TftView) -> PublishResult {
        let mut state = match self.inner.try_lock() {
            Ok(state) => state,
            Err(TryLockError::WouldBlock) => return PublishResult::Busy,
            Err(TryLockError::Poisoned(error)) => error.into_inner(),
        };
        if view.generation <= state.latest_generation {
            return PublishResult::Stale;
        }
        state.latest_generation = view.generation;
        state.pending = Some(view);
        PublishResult::Accepted
    }

    /// Takes the current newest request. No intermediate queue is retained.
    #[must_use]
    pub fn try_take(&self) -> Option<TftView> {
        let mut state = self.inner.try_lock().ok()?;
        state.pending.take()
    }

    /// Returns true only when a newer generation can be observed immediately.
    /// Contention is treated as unknown so a transfer never waits on this lock.
    #[must_use]
    pub fn try_has_newer_than(&self, generation: u64) -> Option<bool> {
        self.inner
            .try_lock()
            .ok()
            .map(|state| state.latest_generation > generation)
    }
}

impl Default for LatestViewSlot {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DisplayHealth {
    consecutive_failures: u8,
    retry_after_ms: u64,
    next_log_ms: u64,
}

impl DisplayHealth {
    #[must_use]
    pub const fn consecutive_failures(self) -> u8 {
        self.consecutive_failures
    }

    #[must_use]
    pub const fn retry_due(self, now_ms: u64) -> bool {
        now_ms >= self.retry_after_ms
    }

    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.retry_after_ms = 0;
    }

    /// Records bounded exponential backoff and returns whether this failure may be logged.
    pub fn record_failure(&mut self, now_ms: u64) -> bool {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        let exponent = u32::from(self.consecutive_failures.min(5));
        let delay_ms = 100_u64.saturating_mul(1_u64 << exponent).min(5_000);
        self.retry_after_ms = now_ms.saturating_add(delay_ms);
        let should_log = now_ms >= self.next_log_ms;
        if should_log {
            self.next_log_ms = now_ms.saturating_add(30_000);
        }
        should_log
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Barrier},
        thread,
    };

    use focus_core::{App, InputEvent, SettingsLoad, default_catalog};

    use super::*;
    use crate::presentation::tft_view;

    fn view(generation: u64) -> TftView {
        let app = App::boot(default_catalog(), SettingsLoad::Missing).0;
        tft_view(&app.snapshot(0), generation)
    }

    #[test]
    fn newer_view_replaces_pending_and_stale_cannot_win() {
        let slot = LatestViewSlot::new();
        assert_eq!(slot.try_publish(view(1)), PublishResult::Accepted);
        assert_eq!(slot.try_publish(view(3)), PublishResult::Accepted);
        assert_eq!(slot.try_publish(view(2)), PublishResult::Stale);
        assert_eq!(slot.try_take().unwrap().generation, 3);
    }

    #[test]
    fn contention_returns_immediately_and_can_be_retried() {
        let slot = LatestViewSlot::new();
        let locked = Arc::clone(&slot.inner);
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let worker = {
            let entered = Arc::clone(&entered);
            let release = Arc::clone(&release);
            thread::spawn(move || {
                let _guard = locked.lock().unwrap();
                entered.wait();
                release.wait();
            })
        };
        entered.wait();
        assert_eq!(slot.try_publish(view(1)), PublishResult::Busy);
        release.wait();
        worker.join().unwrap();
        assert_eq!(slot.try_publish(view(1)), PublishResult::Accepted);
    }

    #[test]
    fn busy_display_boundary_cannot_block_authoritative_app() {
        let slot = LatestViewSlot::new();
        let _guard = slot.inner.lock().unwrap();
        let mut app = App::boot(default_catalog(), SettingsLoad::Missing).0;
        assert!(app.handle(0, InputEvent::Press).render);
        let _ = app.handle(1_000, InputEvent::TimeAdvanced);
        assert_eq!(
            slot.try_publish(tft_view(&app.snapshot(1_000), 1)),
            PublishResult::Busy
        );
        assert_eq!(app.snapshot(1_000).remaining_ms, 1_499_000);
    }

    #[test]
    fn health_backoff_and_log_throttle_are_bounded() {
        let mut health = DisplayHealth::default();
        assert!(health.record_failure(0));
        assert!(!health.record_failure(1));
        assert!(!health.retry_due(100));
        assert!(health.consecutive_failures() >= 2);
        health.record_success();
        assert_eq!(health.consecutive_failures(), 0);
        assert!(health.retry_due(0));
    }
}
