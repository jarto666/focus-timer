use heapless::{Deque, String, Vec};

pub const MAX_PRESET_ID_BYTES: usize = 32;
pub const MAX_PRESET_NAME_BYTES: usize = 32;
pub const MAX_JOURNAL_SEQUENCE: u64 = 9_007_199_254_740_991;

pub type DeviceId = [u8; 16];
pub type JournalEpoch = [u8; 8];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JournalHealth {
    Healthy,
    Degraded,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionOutcome {
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresetSnapshot {
    pub id: String<MAX_PRESET_ID_BYTES>,
    pub name: String<MAX_PRESET_NAME_BYTES>,
    pub planned_duration_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingRecord {
    pub preset: PresetSnapshot,
    pub active_duration_ms: u64,
    pub outcome: SessionOutcome,
    pub started_at_utc_ms: Option<u64>,
    pub ended_at_utc_ms: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JournalRecord {
    pub sequence: u64,
    pub preset: PresetSnapshot,
    pub active_duration_ms: u64,
    pub outcome: SessionOutcome,
    pub started_at_utc_ms: Option<u64>,
    pub ended_at_utc_ms: Option<u64>,
}

impl JournalRecord {
    fn from_pending(sequence: u64, pending: PendingRecord) -> Self {
        Self {
            sequence,
            preset: pending.preset,
            active_duration_ms: pending.active_duration_ms,
            outcome: pending.outcome,
            started_at_utc_ms: pending.started_at_utc_ms,
            ended_at_utc_ms: pending.ended_at_utc_ms,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JournalError {
    ZeroCapacity,
    InvalidHighWater,
    InvalidRetainedSequence,
    RetainedRecordsOutOfOrder,
    TooManyRetainedRecords,
    SequenceExhausted,
    UnexpectedSequence,
    InvalidPageLimit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JournalPage<const PAGE_CAPACITY: usize> {
    pub journal_epoch: JournalEpoch,
    pub oldest_sequence: Option<u64>,
    pub latest_sequence: Option<u64>,
    pub gap: bool,
    pub epoch_changed: bool,
    pub records: Vec<JournalRecord, PAGE_CAPACITY>,
    pub continuation_after_sequence: Option<u64>,
}

/// In-memory model of a persistent fixed-capacity journal.
///
/// `high_water_sequence` is deliberately separate from retained records so
/// sequence positions survive eviction and reboot reconstruction.
#[derive(Debug)]
pub struct Journal<const CAPACITY: usize> {
    device_id: DeviceId,
    epoch: JournalEpoch,
    high_water_sequence: u64,
    health: JournalHealth,
    records: Deque<JournalRecord, CAPACITY>,
}

impl<const CAPACITY: usize> Journal<CAPACITY> {
    /// Creates an empty journal with provisioned stable identity and epoch.
    ///
    /// # Errors
    ///
    /// Returns [`JournalError::ZeroCapacity`] for a zero-sized ring.
    pub fn new(device_id: DeviceId, epoch: JournalEpoch) -> Result<Self, JournalError> {
        if CAPACITY == 0 {
            return Err(JournalError::ZeroCapacity);
        }
        Ok(Self {
            device_id,
            epoch,
            high_water_sequence: 0,
            health: JournalHealth::Healthy,
            records: Deque::new(),
        })
    }

    /// Reconstructs a journal from storage-owned validated inputs.
    ///
    /// # Errors
    ///
    /// Rejects impossible high-water values, non-ascending or out-of-range
    /// retained records, and retained sets larger than the fixed capacity.
    pub fn reconstruct(
        device_id: DeviceId,
        epoch: JournalEpoch,
        high_water_sequence: u64,
        health: JournalHealth,
        retained: impl IntoIterator<Item = JournalRecord>,
    ) -> Result<Self, JournalError> {
        let mut journal = Self::reconstruct_empty(device_id, epoch, high_water_sequence, health)?;
        for record in retained {
            journal.retain_reconstructed(record)?;
        }
        Ok(journal)
    }

    pub(crate) fn reconstruct_empty(
        device_id: DeviceId,
        epoch: JournalEpoch,
        high_water_sequence: u64,
        health: JournalHealth,
    ) -> Result<Self, JournalError> {
        if CAPACITY == 0 {
            return Err(JournalError::ZeroCapacity);
        }
        if high_water_sequence > MAX_JOURNAL_SEQUENCE {
            return Err(JournalError::InvalidHighWater);
        }

        Ok(Self {
            device_id,
            epoch,
            high_water_sequence,
            health,
            records: Deque::new(),
        })
    }

    pub(crate) fn retain_reconstructed(
        &mut self,
        record: JournalRecord,
    ) -> Result<(), JournalError> {
        if record.sequence == 0 || record.sequence > self.high_water_sequence {
            return Err(JournalError::InvalidRetainedSequence);
        }
        if self
            .records
            .back()
            .is_some_and(|previous| record.sequence <= previous.sequence)
        {
            return Err(JournalError::RetainedRecordsOutOfOrder);
        }
        self.records
            .push_back(record)
            .map_err(|_| JournalError::TooManyRetainedRecords)
    }

    #[must_use]
    pub const fn device_id(&self) -> DeviceId {
        self.device_id
    }

    #[must_use]
    pub const fn epoch(&self) -> JournalEpoch {
        self.epoch
    }

    #[must_use]
    pub const fn high_water_sequence(&self) -> u64 {
        self.high_water_sequence
    }

    #[must_use]
    pub const fn health(&self) -> JournalHealth {
        self.health
    }

    pub const fn set_health(&mut self, health: JournalHealth) {
        self.health = health;
    }

    #[must_use]
    pub fn bounds(&self) -> (Option<u64>, Option<u64>) {
        (
            self.records.front().map(|record| record.sequence),
            self.records.back().map(|record| record.sequence),
        )
    }

    /// Assigns the next monotonic sequence and retains the record, evicting the
    /// oldest entry only when the ring is full.
    ///
    /// # Errors
    ///
    /// Returns [`JournalError::SequenceExhausted`] before sequence reuse; the
    /// storage adapter must provision a new epoch to continue.
    pub fn append(&mut self, pending: PendingRecord) -> Result<u64, JournalError> {
        let record = self.prepare(pending)?;
        let sequence = record.sequence;
        self.commit(record)?;
        Ok(sequence)
    }

    /// Builds the next record without changing the in-memory journal. Storage
    /// adapters use this to persist a slot before committing metadata/state.
    ///
    /// # Errors
    ///
    /// Returns [`JournalError::SequenceExhausted`] before sequence reuse.
    pub fn prepare(&self, pending: PendingRecord) -> Result<JournalRecord, JournalError> {
        let sequence = self.next_sequence()?;
        Ok(JournalRecord::from_pending(sequence, pending))
    }

    /// Commits a previously prepared next record to the bounded ring.
    ///
    /// # Errors
    ///
    /// Rejects a record that is not exactly the next monotonic sequence.
    pub fn commit(&mut self, record: JournalRecord) -> Result<(), JournalError> {
        if record.sequence != self.next_sequence()? {
            return Err(JournalError::UnexpectedSequence);
        }

        if self.records.is_full() {
            let _evicted = self.records.pop_front();
        }
        self.records
            .push_back(record)
            .map_err(|_| JournalError::ZeroCapacity)?;
        self.high_water_sequence = self.records.back().map_or(0, |record| record.sequence);
        Ok(())
    }

    fn next_sequence(&self) -> Result<u64, JournalError> {
        self.high_water_sequence
            .checked_add(1)
            .filter(|sequence| *sequence <= MAX_JOURNAL_SEQUENCE)
            .ok_or(JournalError::SequenceExhausted)
    }

    /// Returns a stateless ascending page for a client cursor.
    ///
    /// # Errors
    ///
    /// Returns [`JournalError::InvalidPageLimit`] when `limit` is zero or
    /// exceeds the page container capacity.
    pub fn page<const PAGE_CAPACITY: usize>(
        &self,
        requested_epoch: Option<JournalEpoch>,
        after_sequence: u64,
        limit: usize,
    ) -> Result<JournalPage<PAGE_CAPACITY>, JournalError> {
        if limit == 0 || limit > PAGE_CAPACITY {
            return Err(JournalError::InvalidPageLimit);
        }

        let (oldest_sequence, latest_sequence) = self.bounds();
        let epoch_changed = requested_epoch.is_some_and(|epoch| epoch != self.epoch);
        let effective_after = if epoch_changed { 0 } else { after_sequence };
        let gap = oldest_sequence
            .is_some_and(|oldest| !epoch_changed && effective_after.saturating_add(1) < oldest);

        let mut records = Vec::new();
        for record in self
            .records
            .iter()
            .filter(|record| gap || epoch_changed || record.sequence > effective_after)
        {
            if records.len() == limit {
                break;
            }
            let _ = records.push(record.clone());
        }

        let continuation_after_sequence = records.last().and_then(|last| {
            latest_sequence
                .filter(|latest| last.sequence < *latest)
                .map(|_| last.sequence)
        });

        Ok(JournalPage {
            journal_epoch: self.epoch,
            oldest_sequence,
            latest_sequence,
            gap,
            epoch_changed,
            records,
            continuation_after_sequence,
        })
    }
}
