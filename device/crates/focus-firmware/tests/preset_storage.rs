use focus_core::{MAX_CUSTOM_PRESETS, Preset};
use focus_firmware::preset_storage::{
    CATALOG_DOCUMENT_MAX_BYTES, CATALOG_METADATA_BYTES, CatalogCommitError, PresetCatalogStore,
    commit_catalog, load_catalog,
};
use heapless::Vec as HeaplessVec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fault {
    Injected,
}

#[derive(Default)]
struct MemoryStore {
    slots: [Option<Vec<u8>>; 2],
    metadata: [Option<Vec<u8>>; 2],
    writes: usize,
    fail_on_write: Option<usize>,
}

impl MemoryStore {
    fn maybe_fail(&mut self) -> Result<(), Fault> {
        self.writes += 1;
        if self.fail_on_write == Some(self.writes) {
            return Err(Fault::Injected);
        }
        Ok(())
    }
}

impl PresetCatalogStore for MemoryStore {
    type Error = Fault;

    fn read_slot(&mut self, slot: usize, output: &mut [u8]) -> Result<Option<usize>, Self::Error> {
        Ok(self.slots[slot].as_ref().map(|bytes| {
            output[..bytes.len()].copy_from_slice(bytes);
            bytes.len()
        }))
    }

    fn write_slot(&mut self, slot: usize, document: &[u8]) -> Result<(), Self::Error> {
        self.maybe_fail()?;
        self.slots[slot] = Some(document.to_vec());
        Ok(())
    }

    fn read_metadata(
        &mut self,
        copy: usize,
        output: &mut [u8],
    ) -> Result<Option<usize>, Self::Error> {
        Ok(self.metadata[copy].as_ref().map(|bytes| {
            output[..bytes.len()].copy_from_slice(bytes);
            bytes.len()
        }))
    }

    fn write_metadata(&mut self, copy: usize, metadata: &[u8]) -> Result<(), Self::Error> {
        self.maybe_fail()?;
        self.metadata[copy] = Some(metadata.to_vec());
        Ok(())
    }
}

fn customs() -> HeaplessVec<Preset, MAX_CUSTOM_PRESETS> {
    let mut presets = HeaplessVec::new();
    presets
        .push(Preset::custom("writing", "Writing", 45 * 60_000).unwrap())
        .unwrap();
    presets
}

#[test]
fn first_boot_and_successful_replacement_survive_reload() {
    let mut store = MemoryStore::default();
    assert_eq!(load_catalog(&mut store).unwrap().revision, 0);
    assert!(!load_catalog(&mut store).unwrap().degraded);

    let committed = commit_catalog(&mut store, 0, customs().as_slice()).unwrap();
    assert_eq!(committed.revision, 1);
    let loaded = load_catalog(&mut store).unwrap();
    assert_eq!(loaded.revision, 1);
    assert_eq!(loaded.custom_entries, customs());
    assert!(!loaded.degraded);

    let empty: HeaplessVec<Preset, MAX_CUSTOM_PRESETS> = HeaplessVec::new();
    assert_eq!(
        commit_catalog(&mut store, 1, empty.as_slice())
            .unwrap()
            .revision,
        2
    );
    assert_eq!(load_catalog(&mut store).unwrap().revision, 2);
}

#[test]
fn write_and_verification_failures_never_destroy_the_prior_catalog() {
    let mut store = MemoryStore::default();
    commit_catalog(&mut store, 0, customs().as_slice()).unwrap();

    store.fail_on_write = Some(store.writes + 1);
    assert_eq!(
        commit_catalog(&mut store, 1, &[]),
        Err(CatalogCommitError::Storage(Fault::Injected))
    );
    assert_eq!(load_catalog(&mut store).unwrap().revision, 1);

    store.fail_on_write = None;
    let active_slot = 1;
    store.slots[active_slot].as_mut().unwrap()[5] ^= 0xff;
    let fallback = load_catalog(&mut store).unwrap();
    assert_eq!(fallback.revision, 0);
    assert!(fallback.degraded);
}

#[test]
fn metadata_corruption_recovers_one_valid_slot_as_degraded() {
    let mut store = MemoryStore::default();
    commit_catalog(&mut store, 0, customs().as_slice()).unwrap();
    for metadata in store.metadata.iter_mut().flatten() {
        metadata[0] ^= 0xff;
    }
    let recovered = load_catalog(&mut store).unwrap();
    assert_eq!(recovered.revision, 1);
    assert_eq!(recovered.custom_entries, customs());
    assert!(recovered.degraded);
}

#[test]
fn corrupt_newest_slot_falls_back_to_previous_valid_revision() {
    let mut store = MemoryStore::default();
    commit_catalog(&mut store, 0, customs().as_slice()).unwrap();
    commit_catalog(&mut store, 1, &[]).unwrap();
    store.slots[0].as_mut().unwrap()[6] ^= 0x01;
    for metadata in store.metadata.iter_mut().flatten() {
        metadata[0] ^= 0xff;
    }
    let recovered = load_catalog(&mut store).unwrap();
    assert_eq!(recovered.revision, 1);
    assert!(recovered.degraded);
}

#[test]
fn maximum_document_remains_inside_fixed_storage_bound() {
    let mut entries = HeaplessVec::<Preset, MAX_CUSTOM_PRESETS>::new();
    for index in 0..MAX_CUSTOM_PRESETS {
        entries
            .push(
                Preset::custom(
                    format!("custom-{index:02}-abcdefghijklmnopqrstu").as_str(),
                    format!("Preset {index:02} abcdefghijklmnopqrstu").as_str(),
                    720 * 60_000,
                )
                .unwrap(),
            )
            .unwrap();
    }
    let mut store = MemoryStore::default();
    commit_catalog(&mut store, 0, entries.as_slice()).unwrap();
    assert!(
        store
            .slots
            .iter()
            .flatten()
            .all(|slot| slot.len() <= CATALOG_DOCUMENT_MAX_BYTES)
    );
    assert!(
        store
            .metadata
            .iter()
            .flatten()
            .all(|meta| meta.len() == CATALOG_METADATA_BYTES)
    );
}
