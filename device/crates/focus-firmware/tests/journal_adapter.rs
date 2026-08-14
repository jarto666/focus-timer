use std::{cell::Cell, collections::BTreeMap, rc::Rc, vec::Vec};

use focus_core::{App, InputEvent, SettingsLoad, default_catalog};
use focus_firmware::journal_adapter::{
    FlushOutcome, ObserveError, ObserveOutcome, OutcomeJournalQueue,
};
use focus_sync::{BlobStore, EntropySource, PersistentJournal, VolatileClock};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StoreError;

#[derive(Default)]
struct StoreControl {
    fail_next_write: Cell<bool>,
}

#[derive(Default)]
struct MemoryStore {
    blobs: BTreeMap<std::string::String, Vec<u8>>,
    control: Rc<StoreControl>,
}

impl BlobStore for MemoryStore {
    type Error = StoreError;

    fn read(&mut self, key: &str, output: &mut [u8]) -> Result<Option<usize>, Self::Error> {
        let Some(bytes) = self.blobs.get(key) else {
            return Ok(None);
        };
        output[..bytes.len()].copy_from_slice(bytes);
        Ok(Some(bytes.len()))
    }

    fn write(&mut self, key: &str, bytes: &[u8]) -> Result<(), Self::Error> {
        if self.control.fail_next_write.replace(false) {
            return Err(StoreError);
        }
        self.blobs.insert(key.into(), bytes.into());
        Ok(())
    }
}

#[derive(Default)]
struct Entropy(u8);

impl EntropySource for Entropy {
    type Error = ();

    fn fill(&mut self, output: &mut [u8]) -> Result<(), Self::Error> {
        self.0 += 1;
        output.fill(self.0);
        Ok(())
    }
}

fn app() -> App {
    let catalog = default_catalog();
    App::boot(catalog, SettingsLoad::Missing).0
}

#[test]
fn committed_completion_is_timestamped_and_flushed_once() {
    let mut app = app();
    let mut queue = OutcomeJournalQueue::new();
    let mut clock = VolatileClock::new();
    clock.set_anchor(1_700_000_000_000, 100).unwrap();

    let before = app.snapshot(100).state;
    let effects = app.handle(100, InputEvent::Press);
    let after = app.snapshot(100).state;
    assert_eq!(
        queue.observe(before, after, 100, effects.outcome, clock),
        Ok(ObserveOutcome::NoRecord)
    );

    let end_ms = 100 + app.selected_preset().duration_ms;
    let before = app.snapshot(end_ms).state;
    let effects = app.handle(end_ms, InputEvent::TimeAdvanced);
    let after = app.snapshot(end_ms).state;
    assert_eq!(
        queue.observe(before, after, end_ms, effects.outcome, clock),
        Ok(ObserveOutcome::Queued)
    );

    let mut entropy = Entropy::default();
    let (mut journal, _) =
        PersistentJournal::<_, 4>::initialize(MemoryStore::default(), &mut entropy).unwrap();
    assert_eq!(
        queue.flush_due(end_ms, &mut journal),
        FlushOutcome::Saved { sequence: 1 }
    );
    assert!(!queue.has_pending());
    let page = journal.page::<1>(None, 0, 1).unwrap();
    assert_eq!(page.records.len(), 1);
    assert_eq!(page.records[0].started_at_utc_ms, Some(1_700_000_000_000));
    assert_eq!(
        page.records[0].ended_at_utc_ms,
        Some(1_700_000_000_000 + app.selected_preset().duration_ms)
    );
}

#[test]
fn unknown_clock_stays_unknown_and_pending_slot_is_never_overwritten() {
    let mut app = app();
    let mut queue = OutcomeJournalQueue::new();
    let clock = VolatileClock::new();

    let before = app.snapshot(0).state;
    let start = app.handle(0, InputEvent::Press);
    queue
        .observe(before, app.snapshot(0).state, 0, start.outcome, clock)
        .unwrap();
    let before = app.snapshot(1_000).state;
    let cancel = app.handle(1_000, InputEvent::LongPress);
    queue
        .observe(
            before,
            app.snapshot(1_000).state,
            1_000,
            cancel.outcome,
            clock,
        )
        .unwrap();

    app.handle(2_000, InputEvent::Press);
    let before = app.snapshot(3_000).state;
    let second_cancel = app.handle(3_000, InputEvent::LongPress);
    assert_eq!(
        queue.observe(
            before,
            app.snapshot(3_000).state,
            3_000,
            second_cancel.outcome,
            clock,
        ),
        Err(ObserveError::PendingRecordOccupied)
    );

    let mut entropy = Entropy::default();
    let (mut journal, _) =
        PersistentJournal::<_, 4>::initialize(MemoryStore::default(), &mut entropy).unwrap();
    assert!(matches!(
        queue.flush_due(3_000, &mut journal),
        FlushOutcome::Saved { sequence: 1 }
    ));
    let page = journal.page::<1>(None, 0, 1).unwrap();
    assert_eq!(page.records[0].started_at_utc_ms, None);
    assert_eq!(page.records[0].ended_at_utc_ms, None);
}

#[test]
fn failed_append_retries_the_same_pending_record_with_backoff() {
    let mut app = app();
    let mut queue = OutcomeJournalQueue::new();
    app.handle(0, InputEvent::Press);
    let before = app.snapshot(1_000).state;
    let cancel = app.handle(1_000, InputEvent::LongPress);
    queue
        .observe(
            before,
            app.snapshot(1_000).state,
            1_000,
            cancel.outcome,
            VolatileClock::new(),
        )
        .unwrap();

    let store = MemoryStore::default();
    let control = store.control.clone();
    let mut entropy = Entropy::default();
    let (mut journal, _) = PersistentJournal::<_, 4>::initialize(store, &mut entropy).unwrap();
    control.fail_next_write.set(true);

    assert!(matches!(
        queue.flush_due(1_000, &mut journal),
        FlushOutcome::Failed { .. }
    ));
    assert_eq!(queue.flush_due(1_999, &mut journal), FlushOutcome::Waiting);
    assert_eq!(
        queue.flush_due(2_000, &mut journal),
        FlushOutcome::Saved { sequence: 1 }
    );
    assert_eq!(journal.journal().bounds(), (Some(1), Some(1)));
}
