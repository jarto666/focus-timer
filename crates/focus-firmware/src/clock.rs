use std::time::Instant;

/// Firmware-owned monotonic epoch converted to the core's millisecond unit.
pub struct MonotonicClock {
    epoch: Instant,
}

impl MonotonicClock {
    #[must_use]
    pub fn new() -> Self {
        Self {
            epoch: Instant::now(),
        }
    }

    #[must_use]
    pub fn now_ms(&self) -> u64 {
        u64::try_from(self.epoch.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}
