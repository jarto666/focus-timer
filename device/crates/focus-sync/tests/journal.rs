use focus_sync::{
    Journal, JournalError, JournalHealth, JournalRecord, MAX_JOURNAL_SEQUENCE, PendingRecord,
    PresetSnapshot, SessionOutcome,
};
use heapless::String;

const DEVICE_ID: [u8; 16] = [0x11; 16];
const EPOCH: [u8; 8] = [0x22; 8];

fn text<const N: usize>(value: &str) -> String<N> {
    String::try_from(value).expect("test string fits")
}

fn pending(index: u64) -> PendingRecord {
    PendingRecord {
        preset: PresetSnapshot {
            id: text("focus"),
            name: text("Focus"),
            planned_duration_ms: 1_500_000,
        },
        active_duration_ms: index * 1_000,
        outcome: if index % 2 == 0 {
            SessionOutcome::Completed
        } else {
            SessionOutcome::Cancelled
        },
        started_at_utc_ms: index.checked_mul(10_000),
        ended_at_utc_ms: index.checked_mul(10_000).map(|value| value + 1_000),
    }
}

fn record(sequence: u64) -> JournalRecord {
    let pending = pending(sequence);
    JournalRecord {
        sequence,
        preset: pending.preset,
        active_duration_ms: pending.active_duration_ms,
        outcome: pending.outcome,
        started_at_utc_ms: pending.started_at_utc_ms,
        ended_at_utc_ms: pending.ended_at_utc_ms,
    }
}

#[test]
fn append_wraps_without_reusing_sequences() {
    let mut journal = Journal::<3>::new(DEVICE_ID, EPOCH).unwrap();
    for index in 1..=5 {
        assert_eq!(journal.append(pending(index)).unwrap(), index);
    }

    assert_eq!(journal.device_id(), DEVICE_ID);
    assert_eq!(journal.epoch(), EPOCH);
    assert_eq!(journal.high_water_sequence(), 5);
    assert_eq!(journal.bounds(), (Some(3), Some(5)));
}

#[test]
fn pages_are_ascending_bounded_and_repeatable() {
    let mut journal = Journal::<6>::new(DEVICE_ID, EPOCH).unwrap();
    for index in 1..=5 {
        journal.append(pending(index)).unwrap();
    }

    let first = journal.page::<3>(None, 0, 2).unwrap();
    assert_eq!(
        first
            .records
            .iter()
            .map(|record| record.sequence)
            .collect::<heapless::Vec<_, 3>>()
            .as_slice(),
        &[1, 2]
    );
    assert_eq!(first.continuation_after_sequence, Some(2));
    assert!(!first.gap);
    assert!(!first.epoch_changed);
    assert_eq!(journal.page::<3>(None, 0, 2).unwrap(), first);

    let second = journal.page::<3>(Some(EPOCH), 2, 3).unwrap();
    assert_eq!(
        second
            .records
            .iter()
            .map(|record| record.sequence)
            .collect::<heapless::Vec<_, 3>>()
            .as_slice(),
        &[3, 4, 5]
    );
    assert_eq!(second.continuation_after_sequence, None);

    let current = journal.page::<3>(Some(EPOCH), 5, 3).unwrap();
    assert!(current.records.is_empty());
    assert_eq!(current.bounds(), (Some(1), Some(5)));
}

#[test]
fn stale_cursor_reports_gap_and_starts_at_oldest() {
    let mut journal = Journal::<3>::new(DEVICE_ID, EPOCH).unwrap();
    for index in 1..=5 {
        journal.append(pending(index)).unwrap();
    }

    let page = journal.page::<3>(Some(EPOCH), 1, 3).unwrap();
    assert!(page.gap);
    assert_eq!(
        page.records
            .iter()
            .map(|record| record.sequence)
            .collect::<heapless::Vec<_, 3>>()
            .as_slice(),
        &[3, 4, 5]
    );
}

#[test]
fn changed_epoch_is_a_new_generation_not_a_gap() {
    let mut journal = Journal::<3>::new(DEVICE_ID, EPOCH).unwrap();
    journal.append(pending(1)).unwrap();
    journal.append(pending(2)).unwrap();

    let page = journal.page::<3>(Some([0x99; 8]), 100, 3).unwrap();
    assert!(page.epoch_changed);
    assert!(!page.gap);
    assert_eq!(page.records[0].sequence, 1);
}

#[test]
fn reconstruction_preserves_identity_epoch_health_and_high_water() {
    let mut journal = Journal::<4>::reconstruct(
        DEVICE_ID,
        EPOCH,
        9,
        JournalHealth::Degraded,
        [record(8), record(9)],
    )
    .unwrap();

    assert_eq!(journal.device_id(), DEVICE_ID);
    assert_eq!(journal.epoch(), EPOCH);
    assert_eq!(journal.health(), JournalHealth::Degraded);
    assert_eq!(journal.append(pending(10)).unwrap(), 10);
    assert_eq!(journal.bounds(), (Some(8), Some(10)));
}

#[test]
fn reconstruction_rejects_ambiguous_inputs() {
    assert_eq!(
        Journal::<2>::reconstruct(DEVICE_ID, EPOCH, 1, JournalHealth::Healthy, [record(2)])
            .unwrap_err(),
        JournalError::InvalidRetainedSequence
    );
    assert_eq!(
        Journal::<2>::reconstruct(
            DEVICE_ID,
            EPOCH,
            2,
            JournalHealth::Healthy,
            [record(2), record(1)]
        )
        .unwrap_err(),
        JournalError::RetainedRecordsOutOfOrder
    );
}

#[test]
fn sequence_exhaustion_requires_a_new_epoch() {
    let mut journal = Journal::<1>::reconstruct(
        DEVICE_ID,
        EPOCH,
        MAX_JOURNAL_SEQUENCE,
        JournalHealth::Healthy,
        [],
    )
    .unwrap();
    assert_eq!(
        journal.append(pending(1)).unwrap_err(),
        JournalError::SequenceExhausted
    );
}

#[test]
fn timestamp_presence_is_preserved_without_fabrication() {
    let mut journal = Journal::<2>::new(DEVICE_ID, EPOCH).unwrap();
    let mut unknown = pending(1);
    unknown.started_at_utc_ms = None;
    unknown.ended_at_utc_ms = None;
    journal.append(unknown).unwrap();
    journal.append(pending(2)).unwrap();

    let page = journal.page::<2>(None, 0, 2).unwrap();
    assert_eq!(page.records[0].started_at_utc_ms, None);
    assert_eq!(page.records[0].ended_at_utc_ms, None);
    assert!(page.records[1].started_at_utc_ms.is_some());
    assert!(page.records[1].ended_at_utc_ms.is_some());
}

#[test]
fn page_limit_is_explicitly_bounded() {
    let journal = Journal::<2>::new(DEVICE_ID, EPOCH).unwrap();
    assert_eq!(
        journal.page::<2>(None, 0, 0).unwrap_err(),
        JournalError::InvalidPageLimit
    );
    assert_eq!(
        journal.page::<2>(None, 0, 3).unwrap_err(),
        JournalError::InvalidPageLimit
    );
}

trait PageBounds {
    fn bounds(&self) -> (Option<u64>, Option<u64>);
}

impl<const N: usize> PageBounds for focus_sync::JournalPage<N> {
    fn bounds(&self) -> (Option<u64>, Option<u64>) {
        (self.oldest_sequence, self.latest_sequence)
    }
}
