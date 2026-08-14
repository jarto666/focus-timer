use crate::MAX_JOURNAL_SEQUENCE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockError {
    OutOfRange,
    MonotonicMovedBackwards,
    UtcMovedBackwards,
    Overflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Anchor {
    utc_ms: u64,
    monotonic_ms: u64,
}

/// Volatile UTC-to-monotonic mapping. A fresh instance deliberately has no
/// wall-clock knowledge and no state is intended for persistence.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VolatileClock {
    anchor: Option<Anchor>,
}

impl VolatileClock {
    #[must_use]
    pub const fn new() -> Self {
        Self { anchor: None }
    }

    #[must_use]
    pub const fn is_known(self) -> bool {
        self.anchor.is_some()
    }

    /// Accepts a current UTC value at the supplied monotonic receipt instant.
    ///
    /// # Errors
    ///
    /// Rejects protocol-out-of-range values, a monotonic clock that moved
    /// backwards, or an update that would move known UTC backwards.
    pub fn set_anchor(&mut self, utc_ms: u64, monotonic_ms: u64) -> Result<(), ClockError> {
        if utc_ms > MAX_JOURNAL_SEQUENCE || monotonic_ms > MAX_JOURNAL_SEQUENCE {
            return Err(ClockError::OutOfRange);
        }

        if let Some(anchor) = self.anchor {
            if monotonic_ms < anchor.monotonic_ms {
                return Err(ClockError::MonotonicMovedBackwards);
            }
            let previous_utc = derive(anchor, monotonic_ms)?;
            if utc_ms < previous_utc {
                return Err(ClockError::UtcMovedBackwards);
            }
        }

        self.anchor = Some(Anchor {
            utc_ms,
            monotonic_ms,
        });
        Ok(())
    }

    /// Derives UTC at a monotonic event instant.
    ///
    /// # Errors
    ///
    /// Rejects instants before the active anchor and arithmetic/range overflow.
    pub fn timestamp_ms(self, monotonic_ms: u64) -> Result<Option<u64>, ClockError> {
        self.anchor
            .map(|anchor| derive(anchor, monotonic_ms))
            .transpose()
    }
}

fn derive(anchor: Anchor, monotonic_ms: u64) -> Result<u64, ClockError> {
    let elapsed = monotonic_ms
        .checked_sub(anchor.monotonic_ms)
        .ok_or(ClockError::MonotonicMovedBackwards)?;
    anchor
        .utc_ms
        .checked_add(elapsed)
        .filter(|utc_ms| *utc_ms <= MAX_JOURNAL_SEQUENCE)
        .ok_or(ClockError::Overflow)
}
