use heapless::{String, Vec};

use crate::{
    DeviceId, Journal, JournalEpoch, JournalError, JournalHealth, JournalPage,
    MAX_ENCODED_JOURNAL_RECORD_BYTES, MetadataRecord, PendingRecord, StorageEncodeError,
    StoredJournalRecord, decode_identity, decode_metadata, decode_record, encode_identity,
    encode_metadata, encode_record,
};

pub const SYNC_NAMESPACE: &str = "focus_sync";
const IDENTITY_KEY: &str = "identity";
const METADATA_A_KEY: &str = "meta_a";
const METADATA_B_KEY: &str = "meta_b";
const HEX: &[u8; 16] = b"0123456789abcdef";

pub trait BlobStore {
    type Error;

    /// Reads one blob into `output`, returning its length or `None` when absent.
    ///
    /// # Errors
    ///
    /// Returns a backend-specific read or capacity error.
    fn read(&mut self, key: &str, output: &mut [u8]) -> Result<Option<usize>, Self::Error>;

    /// Atomically replaces one complete blob.
    ///
    /// # Errors
    ///
    /// Returns a backend-specific write/commit error.
    fn write(&mut self, key: &str, bytes: &[u8]) -> Result<(), Self::Error>;
}

pub trait EntropySource {
    type Error;

    /// Fills an identity or epoch with device entropy.
    ///
    /// # Errors
    ///
    /// Returns a backend-specific entropy failure.
    fn fill(&mut self, output: &mut [u8]) -> Result<(), Self::Error>;
}

#[derive(Debug, Eq, PartialEq)]
pub enum InitError<StoreError, EntropyError> {
    UnsupportedCapacity,
    Storage(StoreError),
    Entropy(EntropyError),
    RecordTooLarge,
    Model(JournalError),
    Encode(StorageEncodeError),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum IdentityInit {
    #[default]
    Existing,
    Provisioned,
    Replaced,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EpochInit {
    #[default]
    Existing,
    Provisioned,
    RecoveredFromSlots,
    RotatedAfterAmbiguity,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MetadataInit {
    #[default]
    Clean,
    ReconciledFromSlots,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InitReport {
    pub identity: IdentityInit,
    pub epoch: EpochInit,
    pub metadata: MetadataInit,
    pub invalid_slot_count: usize,
}

#[derive(Debug, Eq, PartialEq)]
pub enum AppendError<StoreError> {
    Model(JournalError),
    Encode(StorageEncodeError),
    SlotWrite(StoreError),
    MetadataWrite(StoreError),
}

/// Owns a bounded journal and commits slot-first updates to a blob backend.
pub struct PersistentJournal<Store, const CAPACITY: usize> {
    store: Store,
    journal: Journal<CAPACITY>,
    metadata_generation: u32,
}

impl<Store, const CAPACITY: usize> PersistentJournal<Store, CAPACITY>
where
    Store: BlobStore,
{
    /// Loads or safely provisions identity, epoch, metadata, and retained slots.
    ///
    /// # Errors
    ///
    /// Returns storage/entropy failures, unsupported capacity, oversized blobs,
    /// or an impossible reconstructed model. Corrupt individual records are
    /// isolated and reported through [`InitReport`] instead of becoming fatal.
    pub fn initialize<Entropy>(
        mut store: Store,
        entropy: &mut Entropy,
    ) -> Result<(Self, InitReport), InitError<Store::Error, Entropy::Error>>
    where
        Entropy: EntropySource,
    {
        if CAPACITY == 0 || CAPACITY > 256 {
            return Err(InitError::UnsupportedCapacity);
        }

        let mut report = InitReport::default();
        let identity_blob = read_blob(&mut store, IDENTITY_KEY).map_err(map_read_error)?;
        let (device_id, identity_is_new) = match identity_blob {
            None => {
                report.identity = IdentityInit::Provisioned;
                (provision_identity(&mut store, entropy)?, true)
            }
            Some(bytes) => {
                if let Ok(device_id) = decode_identity(bytes.as_slice()) {
                    (device_id, false)
                } else {
                    report.identity = IdentityInit::Replaced;
                    (provision_identity(&mut store, entropy)?, true)
                }
            }
        };

        let metadata_a = read_metadata(&mut store, METADATA_A_KEY)?;
        let metadata_b = read_metadata(&mut store, METADATA_B_KEY)?;
        let metadata = select_metadata(metadata_a, metadata_b);
        let mut stored_records =
            read_slots::<_, Entropy::Error, CAPACITY>(&mut store, &mut report)?;

        let (epoch, mut generation, mut high_water_sequence, initialize_metadata) =
            if identity_is_new {
                report.epoch = EpochInit::Provisioned;
                (random_epoch(entropy)?, 0, 0, true)
            } else if let Some(metadata) = metadata {
                (
                    metadata.journal_epoch,
                    metadata.generation,
                    metadata.high_water_sequence,
                    false,
                )
            } else if let Some(epoch) = unique_slot_epoch(stored_records.as_slice()) {
                report.epoch = EpochInit::RecoveredFromSlots;
                let high_water = highest_sequence(stored_records.as_slice(), epoch);
                (epoch, 0, high_water, true)
            } else {
                report.epoch = if stored_records.is_empty() {
                    EpochInit::Provisioned
                } else {
                    EpochInit::RotatedAfterAmbiguity
                };
                (random_epoch(entropy)?, 0, 0, true)
            };

        stored_records.retain(|stored| stored.journal_epoch == epoch);
        stored_records.sort_unstable_by_key(|stored| stored.record.sequence);
        high_water_sequence = high_water_sequence.max(
            stored_records
                .last()
                .map_or(0, |stored| stored.record.sequence),
        );

        if initialize_metadata {
            write_metadata_pair(&mut store, epoch, high_water_sequence)
                .map_err(InitError::Storage)?;
        } else if metadata
            .is_some_and(|metadata| metadata.high_water_sequence != high_water_sequence)
        {
            report.metadata = MetadataInit::ReconciledFromSlots;
            generation = generation.wrapping_add(1);
            write_one_metadata(&mut store, generation, epoch, high_water_sequence)
                .map_err(InitError::Storage)?;
        }

        let health = if report.identity == IdentityInit::Replaced
            || report.epoch == EpochInit::RotatedAfterAmbiguity
            || report.epoch == EpochInit::RecoveredFromSlots
            || report.metadata == MetadataInit::ReconciledFromSlots
            || report.invalid_slot_count > 0
        {
            JournalHealth::Degraded
        } else {
            JournalHealth::Healthy
        };
        let retained = stored_records.into_iter().map(|stored| stored.record);
        let journal = Journal::reconstruct(device_id, epoch, high_water_sequence, health, retained)
            .map_err(InitError::Model)?;

        Ok((
            Self {
                store,
                journal,
                metadata_generation: generation,
            },
            report,
        ))
    }

    #[must_use]
    pub const fn journal(&self) -> &Journal<CAPACITY> {
        &self.journal
    }

    /// Returns one immutable stateless page from the retained journal.
    ///
    /// # Errors
    ///
    /// Rejects a zero or over-capacity page limit.
    pub fn page<const PAGE_CAPACITY: usize>(
        &self,
        requested_epoch: Option<JournalEpoch>,
        after_sequence: u64,
        limit: usize,
    ) -> Result<JournalPage<PAGE_CAPACITY>, JournalError> {
        self.journal.page(requested_epoch, after_sequence, limit)
    }

    /// Persists the next slot, then redundant high-water metadata, and only
    /// then exposes the record through the in-memory journal.
    ///
    /// # Errors
    ///
    /// Distinguishes model/codec errors, slot-write failures, and metadata
    /// failures. A metadata failure leaves the written slot recoverable at boot
    /// while the in-memory high-water remains unchanged for a safe retry.
    pub fn append(&mut self, pending: PendingRecord) -> Result<u64, AppendError<Store::Error>> {
        let record = self.journal.prepare(pending).map_err(AppendError::Model)?;
        let encoded = encode_record(self.journal.epoch(), &record).map_err(AppendError::Encode)?;
        let key = slot_key::<CAPACITY>(record.sequence).map_err(AppendError::Model)?;
        self.store
            .write(key.as_str(), encoded.as_slice())
            .map_err(|error| {
                self.journal.set_health(JournalHealth::Degraded);
                AppendError::SlotWrite(error)
            })?;

        let next_generation = self.metadata_generation.wrapping_add(1);
        let metadata = MetadataRecord {
            generation: next_generation,
            journal_epoch: self.journal.epoch(),
            high_water_sequence: record.sequence,
        };
        let encoded = encode_metadata(metadata).map_err(AppendError::Encode)?;
        self.store
            .write(metadata_key(next_generation), &encoded)
            .map_err(|error| {
                self.journal.set_health(JournalHealth::Degraded);
                AppendError::MetadataWrite(error)
            })?;

        self.journal.commit(record).map_err(AppendError::Model)?;
        self.metadata_generation = next_generation;
        Ok(self.journal.high_water_sequence())
    }

    #[must_use]
    pub fn into_store(self) -> Store {
        self.store
    }
}

enum ReadBlobError<Error> {
    Storage(Error),
    TooLarge,
}

fn map_read_error<StoreError, EntropyError>(
    error: ReadBlobError<StoreError>,
) -> InitError<StoreError, EntropyError> {
    match error {
        ReadBlobError::Storage(error) => InitError::Storage(error),
        ReadBlobError::TooLarge => InitError::RecordTooLarge,
    }
}

fn read_blob<Store>(
    store: &mut Store,
    key: &str,
) -> Result<Option<Vec<u8, MAX_ENCODED_JOURNAL_RECORD_BYTES>>, ReadBlobError<Store::Error>>
where
    Store: BlobStore,
{
    let mut buffer = [0_u8; MAX_ENCODED_JOURNAL_RECORD_BYTES];
    let Some(length) = store
        .read(key, &mut buffer)
        .map_err(ReadBlobError::Storage)?
    else {
        return Ok(None);
    };
    if length > buffer.len() {
        return Err(ReadBlobError::TooLarge);
    }
    let mut bytes = Vec::new();
    bytes
        .extend_from_slice(&buffer[..length])
        .map_err(|()| ReadBlobError::TooLarge)?;
    Ok(Some(bytes))
}

fn provision_identity<Store, Entropy>(
    store: &mut Store,
    entropy: &mut Entropy,
) -> Result<DeviceId, InitError<Store::Error, Entropy::Error>>
where
    Store: BlobStore,
    Entropy: EntropySource,
{
    let mut device_id = [0_u8; 16];
    entropy.fill(&mut device_id).map_err(InitError::Entropy)?;
    store
        .write(IDENTITY_KEY, &encode_identity(device_id))
        .map_err(InitError::Storage)?;
    Ok(device_id)
}

fn random_epoch<StoreError, Entropy>(
    entropy: &mut Entropy,
) -> Result<JournalEpoch, InitError<StoreError, Entropy::Error>>
where
    Entropy: EntropySource,
{
    let mut epoch = [0_u8; 8];
    entropy.fill(&mut epoch).map_err(InitError::Entropy)?;
    Ok(epoch)
}

fn read_metadata<Store, EntropyError>(
    store: &mut Store,
    key: &str,
) -> Result<Option<MetadataRecord>, InitError<Store::Error, EntropyError>>
where
    Store: BlobStore,
{
    let bytes = read_blob(store, key).map_err(map_read_error)?;
    Ok(bytes.and_then(|bytes| decode_metadata(bytes.as_slice()).ok()))
}

fn read_slots<Store, EntropyError, const CAPACITY: usize>(
    store: &mut Store,
    report: &mut InitReport,
) -> Result<Vec<StoredJournalRecord, CAPACITY>, InitError<Store::Error, EntropyError>>
where
    Store: BlobStore,
{
    let mut records = Vec::new();
    for index in 0..CAPACITY {
        let key = slot_key_from_index(index).map_err(InitError::Model)?;
        let Some(bytes) = read_blob(store, key.as_str()).map_err(map_read_error)? else {
            continue;
        };
        let Ok(stored) = decode_record(bytes.as_slice()) else {
            report.invalid_slot_count += 1;
            continue;
        };
        if slot_index::<CAPACITY>(stored.record.sequence) != index {
            report.invalid_slot_count += 1;
            continue;
        }
        let _ = records.push(stored);
    }
    Ok(records)
}

fn select_metadata(a: Option<MetadataRecord>, b: Option<MetadataRecord>) -> Option<MetadataRecord> {
    match (a, b) {
        (Some(a), Some(b)) if a.generation == b.generation => (a == b).then_some(a),
        (Some(a), Some(b)) => {
            let delta = a.generation.wrapping_sub(b.generation);
            if delta < (1 << 31) { Some(a) } else { Some(b) }
        }
        (Some(metadata), None) | (None, Some(metadata)) => Some(metadata),
        (None, None) => None,
    }
}

fn unique_slot_epoch(records: &[StoredJournalRecord]) -> Option<JournalEpoch> {
    let first = records.first()?.journal_epoch;
    records
        .iter()
        .all(|record| record.journal_epoch == first)
        .then_some(first)
}

fn highest_sequence(records: &[StoredJournalRecord], epoch: JournalEpoch) -> u64 {
    records
        .iter()
        .filter(|record| record.journal_epoch == epoch)
        .map(|record| record.record.sequence)
        .max()
        .unwrap_or_default()
}

fn write_metadata_pair<Store>(
    store: &mut Store,
    epoch: JournalEpoch,
    high_water_sequence: u64,
) -> Result<(), Store::Error>
where
    Store: BlobStore,
{
    let encoded = encode_metadata(MetadataRecord {
        generation: 0,
        journal_epoch: epoch,
        high_water_sequence,
    })
    .expect("validated journal positions always encode");
    store.write(METADATA_A_KEY, &encoded)?;
    store.write(METADATA_B_KEY, &encoded)
}

fn write_one_metadata<Store>(
    store: &mut Store,
    generation: u32,
    epoch: JournalEpoch,
    high_water_sequence: u64,
) -> Result<(), Store::Error>
where
    Store: BlobStore,
{
    let encoded = encode_metadata(MetadataRecord {
        generation,
        journal_epoch: epoch,
        high_water_sequence,
    })
    .expect("validated journal positions always encode");
    store.write(metadata_key(generation), &encoded)
}

const fn metadata_key(generation: u32) -> &'static str {
    if generation & 1 == 0 {
        METADATA_A_KEY
    } else {
        METADATA_B_KEY
    }
}

fn slot_key<const CAPACITY: usize>(sequence: u64) -> Result<String<4>, JournalError> {
    if sequence == 0 || CAPACITY == 0 {
        return Err(JournalError::UnexpectedSequence);
    }
    slot_key_from_index(slot_index::<CAPACITY>(sequence))
}

fn slot_index<const CAPACITY: usize>(sequence: u64) -> usize {
    usize::try_from((sequence - 1) % CAPACITY as u64).unwrap_or_default()
}

fn slot_key_from_index(index: usize) -> Result<String<4>, JournalError> {
    let index = u8::try_from(index).map_err(|_| JournalError::UnexpectedSequence)?;
    let mut key = String::new();
    key.push('s')
        .map_err(|()| JournalError::UnexpectedSequence)?;
    key.push(char::from(HEX[usize::from(index >> 4)]))
        .map_err(|()| JournalError::UnexpectedSequence)?;
    key.push(char::from(HEX[usize::from(index & 0x0f)]))
        .map_err(|()| JournalError::UnexpectedSequence)?;
    Ok(key)
}
