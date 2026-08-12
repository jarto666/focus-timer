//! Host-testable settings record and delayed-save policy.

use focus_core::{PresetId, SETTINGS_SCHEMA_VERSION, SettingsLoad, SettingsRecord};
use heapless::{String, Vec};

pub const SAVE_QUIET_PERIOD_MS: u64 = 1_000;
pub const MAX_PRESET_ID_BYTES: usize = 31;
pub const SETTINGS_RECORD_MAX_BYTES: usize = 4 + MAX_PRESET_ID_BYTES;

const MAGIC: [u8; 2] = *b"FT";
const HEADER_BYTES: usize = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredSettings {
    schema_version: u8,
    selected_preset_id: String<MAX_PRESET_ID_BYTES>,
}

impl StoredSettings {
    #[must_use]
    pub fn as_core_load(&self) -> SettingsLoad<'_> {
        SettingsLoad::Record(SettingsRecord {
            schema_version: self.schema_version,
            selected_preset_id: self.selected_preset_id.as_str(),
        })
    }

    #[must_use]
    pub fn current_persisted_id(&self) -> Option<&str> {
        (self.schema_version == SETTINGS_SCHEMA_VERSION).then_some(self.selected_preset_id.as_str())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    BadMagic,
    BadLength,
    InvalidUtf8,
    PresetIdTooLong,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodeError {
    PresetIdTooLong,
}

pub trait SettingsStore {
    type Error;

    /// Reads the complete record into `output`, or returns `None` when absent.
    ///
    /// # Errors
    ///
    /// Returns the backend's read or capacity error.
    fn read(&mut self, output: &mut [u8]) -> Result<Option<usize>, Self::Error>;

    /// Atomically replaces the complete settings record.
    ///
    /// # Errors
    ///
    /// Returns the backend's write or commit error.
    fn write(&mut self, record: &[u8]) -> Result<(), Self::Error>;
}

#[derive(Debug, Eq, PartialEq)]
pub enum LoadError<E> {
    Storage(E),
    RecordTooLarge,
    Decode(DecodeError),
}

#[derive(Debug, Eq, PartialEq)]
pub enum FlushError<E> {
    Encode(EncodeError),
    Storage(E),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueueOutcome {
    Scheduled,
    AlreadyPersisted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlushOutcome {
    NotDue,
    Saved(PresetId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingSelection {
    id: PresetId,
    due_ms: u64,
}

/// Coalesces encoder changes and performs at most one write after quiet time.
pub struct SelectionPersistence {
    persisted_id: Option<String<MAX_PRESET_ID_BYTES>>,
    pending: Option<PendingSelection>,
}

impl SelectionPersistence {
    #[must_use]
    pub fn new(persisted_id: Option<&str>) -> Self {
        let persisted_id = persisted_id.and_then(copy_id);
        Self {
            persisted_id,
            pending: None,
        }
    }

    /// Records the newest in-memory selection without doing flash I/O.
    ///
    /// # Errors
    ///
    /// Returns [`EncodeError::PresetIdTooLong`] when the compile-time identifier
    /// cannot fit in the version-1 record.
    pub fn selection_changed(
        &mut self,
        now_ms: u64,
        id: PresetId,
    ) -> Result<QueueOutcome, EncodeError> {
        if id.as_str().len() > MAX_PRESET_ID_BYTES {
            return Err(EncodeError::PresetIdTooLong);
        }

        if self.persisted_id.as_deref() == Some(id.as_str()) {
            self.pending = None;
            return Ok(QueueOutcome::AlreadyPersisted);
        }

        self.pending = Some(PendingSelection {
            id,
            due_ms: now_ms.saturating_add(SAVE_QUIET_PERIOD_MS),
        });
        Ok(QueueOutcome::Scheduled)
    }

    /// Writes a due selection once. A failed write is deliberately not retried
    /// until a later selection change schedules new work.
    ///
    /// # Errors
    ///
    /// Returns an encode or storage error after consuming the pending write.
    pub fn flush_due<S: SettingsStore>(
        &mut self,
        now_ms: u64,
        store: &mut S,
    ) -> Result<FlushOutcome, FlushError<S::Error>> {
        let Some(pending) = self.pending.filter(|pending| now_ms >= pending.due_ms) else {
            return Ok(FlushOutcome::NotDue);
        };
        self.pending = None;

        let record = encode_record(pending.id).map_err(FlushError::Encode)?;
        store
            .write(record.as_slice())
            .map_err(FlushError::Storage)?;
        self.persisted_id = copy_id(pending.id.as_str());
        Ok(FlushOutcome::Saved(pending.id))
    }
}

/// Loads and decodes one complete record from a storage implementation.
///
/// # Errors
///
/// Returns a storage error, an oversized-record error, or a decode error.
pub fn load_settings<S: SettingsStore>(
    store: &mut S,
) -> Result<Option<StoredSettings>, LoadError<S::Error>> {
    let mut bytes = [0_u8; SETTINGS_RECORD_MAX_BYTES];
    let Some(length) = store.read(&mut bytes).map_err(LoadError::Storage)? else {
        return Ok(None);
    };
    if length > bytes.len() {
        return Err(LoadError::RecordTooLarge);
    }
    decode_record(&bytes[..length])
        .map(Some)
        .map_err(LoadError::Decode)
}

fn encode_record(id: PresetId) -> Result<Vec<u8, SETTINGS_RECORD_MAX_BYTES>, EncodeError> {
    let id_bytes = id.as_str().as_bytes();
    let id_length = u8::try_from(id_bytes.len()).map_err(|_| EncodeError::PresetIdTooLong)?;
    if id_bytes.len() > MAX_PRESET_ID_BYTES {
        return Err(EncodeError::PresetIdTooLong);
    }

    let mut record = Vec::new();
    record
        .extend_from_slice(&MAGIC)
        .map_err(|()| EncodeError::PresetIdTooLong)?;
    record
        .push(SETTINGS_SCHEMA_VERSION)
        .map_err(|_| EncodeError::PresetIdTooLong)?;
    record
        .push(id_length)
        .map_err(|_| EncodeError::PresetIdTooLong)?;
    record
        .extend_from_slice(id_bytes)
        .map_err(|()| EncodeError::PresetIdTooLong)?;
    Ok(record)
}

fn decode_record(bytes: &[u8]) -> Result<StoredSettings, DecodeError> {
    if bytes.len() < HEADER_BYTES {
        return Err(DecodeError::BadLength);
    }
    if bytes[..MAGIC.len()] != MAGIC {
        return Err(DecodeError::BadMagic);
    }

    let id_length = usize::from(bytes[3]);
    if id_length > MAX_PRESET_ID_BYTES {
        return Err(DecodeError::PresetIdTooLong);
    }
    if bytes.len() != HEADER_BYTES + id_length {
        return Err(DecodeError::BadLength);
    }

    let id = core::str::from_utf8(&bytes[HEADER_BYTES..]).map_err(|_| DecodeError::InvalidUtf8)?;
    let mut selected_preset_id = String::new();
    selected_preset_id
        .push_str(id)
        .map_err(|()| DecodeError::PresetIdTooLong)?;

    Ok(StoredSettings {
        schema_version: bytes[2],
        selected_preset_id,
    })
}

fn copy_id(id: &str) -> Option<String<MAX_PRESET_ID_BYTES>> {
    let mut copy = String::new();
    copy.push_str(id).ok().map(|()| copy)
}
