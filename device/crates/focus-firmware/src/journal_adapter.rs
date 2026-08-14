use focus_core::{SessionOutcome as CoreOutcome, SessionOutcomeKind, ViewState};
use focus_sync::{
    AppendError, BlobStore, PendingRecord, PersistentJournal, PresetSnapshot, SessionOutcome,
    VolatileClock,
};
use heapless::String;

const INITIAL_RETRY_MS: u64 = 1_000;
const MAX_RETRY_MS: u64 = 60_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObserveOutcome {
    NoRecord,
    Queued,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObserveError {
    PendingRecordOccupied,
    PresetSnapshotTooLong,
}

#[derive(Debug, Eq, PartialEq)]
pub enum FlushOutcome<Error> {
    Idle,
    Waiting,
    Saved { sequence: u64 },
    Failed { error: AppendError<Error> },
}

/// Tracks optional session-start UTC and retains at most one failed outcome for
/// bounded retry outside the timer state transition.
#[derive(Debug, Default)]
pub struct OutcomeJournalQueue {
    started_at_utc_ms: Option<u64>,
    pending: Option<PendingRecord>,
    retry_at_ms: u64,
    retry_delay_ms: u64,
}

impl OutcomeJournalQueue {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            started_at_utc_ms: None,
            pending: None,
            retry_at_ms: 0,
            retry_delay_ms: INITIAL_RETRY_MS,
        }
    }

    /// Observes a committed transition and copies any semantic outcome into
    /// the single bounded pending slot.
    ///
    /// # Errors
    ///
    /// Rejects preset text outside the persistent schema or a second outcome
    /// while an earlier failed append is still pending.
    pub fn observe(
        &mut self,
        before: ViewState,
        after: ViewState,
        now_ms: u64,
        outcome: Option<CoreOutcome>,
        clock: VolatileClock,
    ) -> Result<ObserveOutcome, ObserveError> {
        if before == ViewState::Idle && after == ViewState::Running {
            self.started_at_utc_ms = clock.timestamp_ms(now_ms).ok().flatten();
        }

        let Some(outcome) = outcome else {
            return Ok(ObserveOutcome::NoRecord);
        };
        if self.pending.is_some() {
            self.started_at_utc_ms = None;
            return Err(ObserveError::PendingRecordOccupied);
        }

        let id = String::try_from(outcome.preset.id.as_str())
            .map_err(|()| ObserveError::PresetSnapshotTooLong)?;
        let name = String::try_from(outcome.preset.name.as_str())
            .map_err(|()| ObserveError::PresetSnapshotTooLong)?;
        self.pending = Some(PendingRecord {
            preset: PresetSnapshot {
                id,
                name,
                planned_duration_ms: outcome.planned_duration_ms,
            },
            active_duration_ms: outcome.active_duration_ms,
            outcome: match outcome.kind {
                SessionOutcomeKind::Completed => SessionOutcome::Completed,
                SessionOutcomeKind::Cancelled => SessionOutcome::Cancelled,
            },
            started_at_utc_ms: self.started_at_utc_ms.take(),
            ended_at_utc_ms: clock.timestamp_ms(now_ms).ok().flatten(),
        });
        self.retry_at_ms = now_ms;
        self.retry_delay_ms = INITIAL_RETRY_MS;
        Ok(ObserveOutcome::Queued)
    }

    #[must_use]
    pub const fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Attempts one due append without ever blocking or mutating timer state.
    ///
    /// A failure keeps the same record and increases a bounded retry delay.
    pub fn flush_due<Store, const CAPACITY: usize>(
        &mut self,
        now_ms: u64,
        journal: &mut PersistentJournal<Store, CAPACITY>,
    ) -> FlushOutcome<Store::Error>
    where
        Store: BlobStore,
    {
        let Some(pending) = self.pending.as_ref() else {
            return FlushOutcome::Idle;
        };
        if now_ms < self.retry_at_ms {
            return FlushOutcome::Waiting;
        }

        match journal.append(pending.clone()) {
            Ok(sequence) => {
                self.pending = None;
                self.retry_delay_ms = INITIAL_RETRY_MS;
                FlushOutcome::Saved { sequence }
            }
            Err(error) => {
                self.retry_at_ms = now_ms.saturating_add(self.retry_delay_ms);
                self.retry_delay_ms = self.retry_delay_ms.saturating_mul(2).min(MAX_RETRY_MS);
                FlushOutcome::Failed { error }
            }
        }
    }
}
