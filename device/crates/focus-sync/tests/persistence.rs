use std::{collections::BTreeMap, vec::Vec};

use focus_sync::{
    AppendError, BlobStore, EntropySource, EpochInit, IdentityInit, JournalHealth, JournalRecord,
    MetadataInit, PendingRecord, PersistentJournal, PresetSnapshot, SessionOutcome, encode_record,
};
use heapless::String;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StoreError {
    Injected,
    TooLarge,
}

#[derive(Default)]
struct MemoryStore {
    blobs: BTreeMap<std::string::String, Vec<u8>>,
    fail_once_key: Option<std::string::String>,
}

impl MemoryStore {
    fn fail_once(&mut self, key: &str) {
        self.fail_once_key = Some(key.into());
    }

    fn corrupt(&mut self, key: &str) {
        self.blobs.get_mut(key).unwrap()[0] ^= 0xff;
    }
}

impl BlobStore for MemoryStore {
    type Error = StoreError;

    fn read(&mut self, key: &str, output: &mut [u8]) -> Result<Option<usize>, Self::Error> {
        let Some(bytes) = self.blobs.get(key) else {
            return Ok(None);
        };
        if bytes.len() > output.len() {
            return Err(StoreError::TooLarge);
        }
        output[..bytes.len()].copy_from_slice(bytes);
        Ok(Some(bytes.len()))
    }

    fn write(&mut self, key: &str, bytes: &[u8]) -> Result<(), Self::Error> {
        if self.fail_once_key.as_deref() == Some(key) {
            self.fail_once_key = None;
            return Err(StoreError::Injected);
        }
        self.blobs.insert(key.into(), bytes.into());
        Ok(())
    }
}

#[derive(Default)]
struct DeterministicEntropy(u8);

impl EntropySource for DeterministicEntropy {
    type Error = ();

    fn fill(&mut self, output: &mut [u8]) -> Result<(), Self::Error> {
        self.0 = self.0.wrapping_add(1);
        output.fill(self.0);
        Ok(())
    }
}

fn text<const N: usize>(value: &str) -> String<N> {
    String::try_from(value).unwrap()
}

fn pending(index: u64) -> PendingRecord {
    PendingRecord {
        preset: PresetSnapshot {
            id: text("focus"),
            name: text("Focus"),
            planned_duration_ms: 10_000,
        },
        active_duration_ms: index * 1_000,
        outcome: SessionOutcome::Cancelled,
        started_at_utc_ms: None,
        ended_at_utc_ms: None,
    }
}

#[test]
fn first_boot_provisions_identity_epoch_and_redundant_metadata() {
    let store = MemoryStore::default();
    let mut entropy = DeterministicEntropy::default();
    let (persistent, report) = PersistentJournal::<_, 64>::initialize(store, &mut entropy).unwrap();

    assert_eq!(report.identity, IdentityInit::Provisioned);
    assert_eq!(report.epoch, EpochInit::Provisioned);
    assert_eq!(persistent.journal().device_id(), [1; 16]);
    assert_eq!(persistent.journal().epoch(), [2; 8]);
    let store = persistent.into_store();
    assert!(store.blobs.contains_key("identity"));
    assert!(store.blobs.contains_key("meta_a"));
    assert!(store.blobs.contains_key("meta_b"));
}

#[test]
fn append_reboot_and_eviction_preserve_monotonic_position() {
    let mut entropy = DeterministicEntropy::default();
    let (mut persistent, _) =
        PersistentJournal::<_, 3>::initialize(MemoryStore::default(), &mut entropy).unwrap();
    let device_id = persistent.journal().device_id();
    let epoch = persistent.journal().epoch();
    for index in 1..=5 {
        assert_eq!(persistent.append(pending(index)).unwrap(), index);
    }
    assert_eq!(persistent.journal().bounds(), (Some(3), Some(5)));

    let store = persistent.into_store();
    let (mut rebooted, report) =
        PersistentJournal::<_, 3>::initialize(store, &mut entropy).unwrap();
    assert_eq!(report.identity, IdentityInit::Existing);
    assert_eq!(rebooted.journal().device_id(), device_id);
    assert_eq!(rebooted.journal().epoch(), epoch);
    assert_eq!(rebooted.journal().bounds(), (Some(3), Some(5)));
    assert_eq!(rebooted.append(pending(6)).unwrap(), 6);
}

#[test]
fn slot_written_before_failed_metadata_is_reconciled_on_reboot() {
    let mut entropy = DeterministicEntropy::default();
    let (persistent, _) =
        PersistentJournal::<_, 4>::initialize(MemoryStore::default(), &mut entropy).unwrap();
    let mut store = persistent.into_store();
    store.fail_once("meta_b");
    let (mut persistent, _) = PersistentJournal::<_, 4>::initialize(store, &mut entropy).unwrap();

    assert_eq!(
        persistent.append(pending(1)).unwrap_err(),
        AppendError::MetadataWrite(StoreError::Injected)
    );
    assert_eq!(persistent.journal().high_water_sequence(), 0);

    let store = persistent.into_store();
    let (rebooted, report) = PersistentJournal::<_, 4>::initialize(store, &mut entropy).unwrap();
    assert_eq!(report.metadata, MetadataInit::ReconciledFromSlots);
    assert_eq!(rebooted.journal().high_water_sequence(), 1);
    assert_eq!(rebooted.journal().bounds(), (Some(1), Some(1)));
}

#[test]
fn corrupt_slot_is_isolated_and_marks_journal_degraded() {
    let mut entropy = DeterministicEntropy::default();
    let (mut persistent, _) =
        PersistentJournal::<_, 4>::initialize(MemoryStore::default(), &mut entropy).unwrap();
    persistent.append(pending(1)).unwrap();
    let mut store = persistent.into_store();
    store.corrupt("s00");

    let (rebooted, report) = PersistentJournal::<_, 4>::initialize(store, &mut entropy).unwrap();
    assert_eq!(report.invalid_slot_count, 1);
    assert_eq!(rebooted.journal().health(), JournalHealth::Degraded);
    assert_eq!(rebooted.journal().bounds(), (None, None));
    assert_eq!(rebooted.journal().high_water_sequence(), 1);
}

#[test]
fn missing_metadata_is_recovered_from_one_slot_epoch() {
    let mut entropy = DeterministicEntropy::default();
    let (mut persistent, _) =
        PersistentJournal::<_, 4>::initialize(MemoryStore::default(), &mut entropy).unwrap();
    persistent.append(pending(1)).unwrap();
    let mut store = persistent.into_store();
    store.blobs.remove("meta_a");
    store.blobs.remove("meta_b");

    let (rebooted, report) = PersistentJournal::<_, 4>::initialize(store, &mut entropy).unwrap();
    assert_eq!(report.epoch, EpochInit::RecoveredFromSlots);
    assert_eq!(rebooted.journal().high_water_sequence(), 1);
    assert_eq!(rebooted.journal().health(), JournalHealth::Degraded);
}

#[test]
fn corrupt_newest_metadata_falls_back_and_reconciles_from_the_committed_slot() {
    let mut entropy = DeterministicEntropy::default();
    let (mut persistent, _) =
        PersistentJournal::<_, 4>::initialize(MemoryStore::default(), &mut entropy).unwrap();
    persistent.append(pending(1)).unwrap();
    let mut store = persistent.into_store();
    store.corrupt("meta_b");

    let (rebooted, report) = PersistentJournal::<_, 4>::initialize(store, &mut entropy).unwrap();
    assert_eq!(report.epoch, EpochInit::Existing);
    assert_eq!(report.metadata, MetadataInit::ReconciledFromSlots);
    assert_eq!(rebooted.journal().high_water_sequence(), 1);
    assert_eq!(rebooted.journal().bounds(), (Some(1), Some(1)));
}

#[test]
fn corrupt_metadata_pair_recovers_the_unique_epoch_from_slots() {
    let mut entropy = DeterministicEntropy::default();
    let (mut persistent, _) =
        PersistentJournal::<_, 4>::initialize(MemoryStore::default(), &mut entropy).unwrap();
    persistent.append(pending(1)).unwrap();
    let mut store = persistent.into_store();
    store.corrupt("meta_a");
    store.corrupt("meta_b");

    let (rebooted, report) = PersistentJournal::<_, 4>::initialize(store, &mut entropy).unwrap();
    assert_eq!(report.epoch, EpochInit::RecoveredFromSlots);
    assert_eq!(rebooted.journal().health(), JournalHealth::Degraded);
    assert_eq!(rebooted.journal().bounds(), (Some(1), Some(1)));
}

#[test]
fn ambiguous_slot_epochs_rotate_to_a_clean_epoch() {
    let mut entropy = DeterministicEntropy::default();
    let (mut persistent, _) =
        PersistentJournal::<_, 4>::initialize(MemoryStore::default(), &mut entropy).unwrap();
    persistent.append(pending(1)).unwrap();
    let old_epoch = persistent.journal().epoch();
    let mut store = persistent.into_store();
    store.blobs.remove("meta_a");
    store.blobs.remove("meta_b");
    let second_epoch = [0xa5; 8];
    let second = JournalRecord {
        sequence: 2,
        preset: pending(2).preset,
        active_duration_ms: 2_000,
        outcome: SessionOutcome::Cancelled,
        started_at_utc_ms: None,
        ended_at_utc_ms: None,
    };
    store.blobs.insert(
        "s01".into(),
        encode_record(second_epoch, &second).unwrap().to_vec(),
    );

    let (rebooted, report) = PersistentJournal::<_, 4>::initialize(store, &mut entropy).unwrap();
    assert_eq!(report.epoch, EpochInit::RotatedAfterAmbiguity);
    assert_ne!(rebooted.journal().epoch(), old_epoch);
    assert_ne!(rebooted.journal().epoch(), second_epoch);
    assert_eq!(rebooted.journal().health(), JournalHealth::Degraded);
    assert_eq!(rebooted.journal().high_water_sequence(), 0);
    assert_eq!(rebooted.journal().bounds(), (None, None));
}

#[test]
fn lost_identity_creates_a_new_logical_device_and_epoch() {
    let mut entropy = DeterministicEntropy::default();
    let (mut persistent, _) =
        PersistentJournal::<_, 4>::initialize(MemoryStore::default(), &mut entropy).unwrap();
    let old_device = persistent.journal().device_id();
    let old_epoch = persistent.journal().epoch();
    persistent.append(pending(1)).unwrap();
    let mut store = persistent.into_store();
    store.blobs.remove("identity");

    let (rebooted, report) = PersistentJournal::<_, 4>::initialize(store, &mut entropy).unwrap();
    assert_eq!(report.identity, IdentityInit::Provisioned);
    assert_ne!(rebooted.journal().device_id(), old_device);
    assert_ne!(rebooted.journal().epoch(), old_epoch);
    assert_eq!(rebooted.journal().bounds(), (None, None));
}

#[test]
fn sync_writes_do_not_touch_unrelated_settings_key() {
    let mut store = MemoryStore::default();
    store.blobs.insert("settings".into(), b"existing".to_vec());
    let mut entropy = DeterministicEntropy::default();
    let (mut persistent, _) = PersistentJournal::<_, 4>::initialize(store, &mut entropy).unwrap();
    persistent.append(pending(1)).unwrap();
    let store = persistent.into_store();

    assert_eq!(store.blobs.get("settings").unwrap(), b"existing");
}
