//! Recoverable two-slot persistence for the complete custom preset document.

use focus_core::{Catalog, MAX_CUSTOM_PRESETS, Preset, default_presets};
use focus_protocol::crc32_iso_hdlc;
use heapless::Vec;

pub const PRESET_NAMESPACE: &str = "focus_presets";
pub const CATALOG_SCHEMA_VERSION: u8 = 1;
pub const CATALOG_SLOT_COUNT: usize = 2;
pub const CATALOG_METADATA_COPIES: usize = 2;
pub const CATALOG_DOCUMENT_MAX_BYTES: usize = 592;
pub const CATALOG_METADATA_BYTES: usize = 16;

const DOCUMENT_MAGIC: [u8; 2] = *b"FP";
const METADATA_MAGIC: [u8; 2] = *b"FM";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredPresetCatalog {
    pub revision: u64,
    pub custom_entries: Vec<Preset, MAX_CUSTOM_PRESETS>,
    pub degraded: bool,
}

impl StoredPresetCatalog {
    #[must_use]
    pub fn first_boot() -> Self {
        Self {
            revision: 0,
            custom_entries: Vec::new(),
            degraded: false,
        }
    }
}

#[allow(clippy::missing_errors_doc)]
pub trait PresetCatalogStore {
    type Error;

    fn read_slot(&mut self, slot: usize, output: &mut [u8]) -> Result<Option<usize>, Self::Error>;
    fn write_slot(&mut self, slot: usize, document: &[u8]) -> Result<(), Self::Error>;
    fn read_metadata(
        &mut self,
        copy: usize,
        output: &mut [u8],
    ) -> Result<Option<usize>, Self::Error>;
    fn write_metadata(&mut self, copy: usize, metadata: &[u8]) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogDocumentError {
    InvalidMagic,
    UnsupportedVersion,
    InvalidLength,
    InvalidChecksum,
    InvalidUtf8,
    InvalidCatalog,
    RevisionExhausted,
    CapacityExceeded,
}

#[derive(Debug, Eq, PartialEq)]
pub enum CatalogLoadError<E> {
    Storage(E),
}

#[derive(Debug, Eq, PartialEq)]
pub enum CatalogCommitError<E> {
    Storage(E),
    Document(CatalogDocumentError),
    VerificationFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Metadata {
    slot: usize,
    revision: u64,
}

/// Loads the newest uniquely valid document, recovering from corrupt metadata.
///
/// # Errors
///
/// Returns the underlying storage error; corrupt records are recovered or degraded in-band.
pub fn load_catalog<S: PresetCatalogStore>(
    store: &mut S,
) -> Result<StoredPresetCatalog, CatalogLoadError<S::Error>> {
    let mut valid_slots: [Option<StoredPresetCatalog>; 2] = [None, None];
    let mut saw_slot_data = false;
    for (slot, destination) in valid_slots.iter_mut().enumerate() {
        let mut bytes = [0; CATALOG_DOCUMENT_MAX_BYTES];
        if let Some(length) = store
            .read_slot(slot, &mut bytes)
            .map_err(CatalogLoadError::Storage)?
        {
            saw_slot_data = true;
            if length <= bytes.len() {
                *destination = decode_document(&bytes[..length]).ok();
            }
        }
    }

    let mut metadata = [None, None];
    let mut saw_metadata = false;
    for (copy, destination) in metadata.iter_mut().enumerate() {
        let mut bytes = [0; CATALOG_METADATA_BYTES];
        if let Some(length) = store
            .read_metadata(copy, &mut bytes)
            .map_err(CatalogLoadError::Storage)?
        {
            saw_metadata = true;
            if length <= bytes.len() {
                *destination = decode_metadata(&bytes[..length]).ok();
            }
        }
    }

    let mut selected: Option<(StoredPresetCatalog, bool)> = None;
    for marker in metadata.into_iter().flatten() {
        let Some(document) = valid_slots.get(marker.slot).and_then(Clone::clone) else {
            continue;
        };
        if document.revision != marker.revision {
            continue;
        }
        if selected
            .as_ref()
            .is_none_or(|(current, _)| document.revision > current.revision)
        {
            selected = Some((document, false));
        }
    }
    if let Some((mut document, degraded)) = selected {
        document.degraded = degraded;
        return Ok(document);
    }

    let mut candidates = valid_slots.into_iter().flatten();
    let first = candidates.next();
    let second = candidates.next();
    let recovered = match (first, second) {
        (Some(first), Some(second)) if first.revision != second.revision => {
            Some(if first.revision > second.revision {
                first
            } else {
                second
            })
        }
        (Some(first), None) => Some(first),
        _ => None,
    };
    if let Some(mut document) = recovered {
        document.degraded = true;
        return Ok(document);
    }

    Ok(StoredPresetCatalog {
        degraded: saw_slot_data || saw_metadata,
        ..StoredPresetCatalog::first_boot()
    })
}

/// Writes and verifies the inactive slot before advancing both metadata copies.
///
/// # Errors
///
/// Returns storage, document-validation, or read-back verification failures without replacing
/// the previously committed catalog.
pub fn commit_catalog<S: PresetCatalogStore>(
    store: &mut S,
    current_revision: u64,
    custom_entries: &[Preset],
) -> Result<StoredPresetCatalog, CatalogCommitError<S::Error>> {
    validate_custom_entries(custom_entries).map_err(CatalogCommitError::Document)?;
    let revision = current_revision
        .checked_add(1)
        .ok_or(CatalogCommitError::Document(
            CatalogDocumentError::RevisionExhausted,
        ))?;
    let loaded = load_catalog(store)
        .map_err(|CatalogLoadError::Storage(error)| CatalogCommitError::Storage(error))?;
    let active_slot = (loaded.revision & 1) as usize;
    let inactive_slot = 1 - active_slot;
    let document =
        encode_document(revision, custom_entries).map_err(CatalogCommitError::Document)?;
    store
        .write_slot(inactive_slot, document.as_slice())
        .map_err(CatalogCommitError::Storage)?;

    let mut verify = [0; CATALOG_DOCUMENT_MAX_BYTES];
    let verified = store
        .read_slot(inactive_slot, &mut verify)
        .map_err(CatalogCommitError::Storage)?
        .and_then(|length| decode_document(&verify[..length]).ok())
        .is_some_and(|decoded| {
            decoded.revision == revision && decoded.custom_entries == custom_entries
        });
    if !verified {
        return Err(CatalogCommitError::VerificationFailed);
    }

    let metadata = encode_metadata(Metadata {
        slot: inactive_slot,
        revision,
    });
    for copy in 0..CATALOG_METADATA_COPIES {
        store
            .write_metadata(copy, metadata.as_slice())
            .map_err(CatalogCommitError::Storage)?;
    }

    let mut owned = Vec::new();
    for preset in custom_entries {
        owned
            .push(preset.clone())
            .map_err(|_| CatalogCommitError::Document(CatalogDocumentError::CapacityExceeded))?;
    }
    Ok(StoredPresetCatalog {
        revision,
        custom_entries: owned,
        degraded: false,
    })
}

fn validate_custom_entries(custom_entries: &[Preset]) -> Result<(), CatalogDocumentError> {
    let mut combined = default_presets();
    for preset in custom_entries {
        combined
            .push(preset.clone())
            .map_err(|_| CatalogDocumentError::CapacityExceeded)?;
    }
    Catalog::combined(combined.as_slice())
        .map(|_| ())
        .map_err(|_| CatalogDocumentError::InvalidCatalog)
}

fn encode_document(
    revision: u64,
    custom_entries: &[Preset],
) -> Result<Vec<u8, CATALOG_DOCUMENT_MAX_BYTES>, CatalogDocumentError> {
    let mut output = Vec::new();
    output
        .extend_from_slice(&DOCUMENT_MAGIC)
        .map_err(|()| CatalogDocumentError::CapacityExceeded)?;
    output
        .push(CATALOG_SCHEMA_VERSION)
        .map_err(|_| CatalogDocumentError::CapacityExceeded)?;
    output
        .extend_from_slice(&revision.to_be_bytes())
        .map_err(|()| CatalogDocumentError::CapacityExceeded)?;
    output
        .push(
            u8::try_from(custom_entries.len())
                .map_err(|_| CatalogDocumentError::CapacityExceeded)?,
        )
        .map_err(|_| CatalogDocumentError::CapacityExceeded)?;
    for preset in custom_entries {
        let id = preset.id.as_str().as_bytes();
        let name = preset.name.as_bytes();
        output
            .push(u8::try_from(id.len()).map_err(|_| CatalogDocumentError::CapacityExceeded)?)
            .map_err(|_| CatalogDocumentError::CapacityExceeded)?;
        output
            .extend_from_slice(id)
            .map_err(|()| CatalogDocumentError::CapacityExceeded)?;
        output
            .push(u8::try_from(name.len()).map_err(|_| CatalogDocumentError::CapacityExceeded)?)
            .map_err(|_| CatalogDocumentError::CapacityExceeded)?;
        output
            .extend_from_slice(name)
            .map_err(|()| CatalogDocumentError::CapacityExceeded)?;
        let duration =
            u32::try_from(preset.duration_ms).map_err(|_| CatalogDocumentError::InvalidCatalog)?;
        output
            .extend_from_slice(&duration.to_be_bytes())
            .map_err(|()| CatalogDocumentError::CapacityExceeded)?;
    }
    let checksum = crc32_iso_hdlc(output.as_slice());
    output
        .extend_from_slice(&checksum.to_be_bytes())
        .map_err(|()| CatalogDocumentError::CapacityExceeded)?;
    Ok(output)
}

fn decode_document(bytes: &[u8]) -> Result<StoredPresetCatalog, CatalogDocumentError> {
    if bytes.len() < 16 || bytes[..2] != DOCUMENT_MAGIC {
        return Err(CatalogDocumentError::InvalidMagic);
    }
    if bytes[2] != CATALOG_SCHEMA_VERSION {
        return Err(CatalogDocumentError::UnsupportedVersion);
    }
    let (payload, checksum_bytes) = bytes.split_at(bytes.len() - 4);
    let expected = u32::from_be_bytes(
        checksum_bytes
            .try_into()
            .map_err(|_| CatalogDocumentError::InvalidLength)?,
    );
    if crc32_iso_hdlc(payload) != expected {
        return Err(CatalogDocumentError::InvalidChecksum);
    }
    let revision = u64::from_be_bytes(
        bytes[3..11]
            .try_into()
            .map_err(|_| CatalogDocumentError::InvalidLength)?,
    );
    if revision == 0 {
        return Err(CatalogDocumentError::InvalidCatalog);
    }
    let count = usize::from(bytes[11]);
    if count > MAX_CUSTOM_PRESETS {
        return Err(CatalogDocumentError::InvalidCatalog);
    }
    let mut cursor = 12;
    let mut custom_entries = Vec::new();
    for _ in 0..count {
        let id_length = *payload
            .get(cursor)
            .ok_or(CatalogDocumentError::InvalidLength)? as usize;
        cursor += 1;
        let id = core::str::from_utf8(
            payload
                .get(cursor..cursor + id_length)
                .ok_or(CatalogDocumentError::InvalidLength)?,
        )
        .map_err(|_| CatalogDocumentError::InvalidUtf8)?;
        cursor += id_length;
        let name_length = *payload
            .get(cursor)
            .ok_or(CatalogDocumentError::InvalidLength)? as usize;
        cursor += 1;
        let name = core::str::from_utf8(
            payload
                .get(cursor..cursor + name_length)
                .ok_or(CatalogDocumentError::InvalidLength)?,
        )
        .map_err(|_| CatalogDocumentError::InvalidUtf8)?;
        cursor += name_length;
        let duration = u32::from_be_bytes(
            payload
                .get(cursor..cursor + 4)
                .ok_or(CatalogDocumentError::InvalidLength)?
                .try_into()
                .map_err(|_| CatalogDocumentError::InvalidLength)?,
        );
        cursor += 4;
        custom_entries
            .push(
                Preset::custom(id, name, u64::from(duration))
                    .map_err(|_| CatalogDocumentError::InvalidCatalog)?,
            )
            .map_err(|_| CatalogDocumentError::CapacityExceeded)?;
    }
    if cursor != payload.len() {
        return Err(CatalogDocumentError::InvalidLength);
    }
    validate_custom_entries(custom_entries.as_slice())?;
    Ok(StoredPresetCatalog {
        revision,
        custom_entries,
        degraded: false,
    })
}

fn encode_metadata(metadata: Metadata) -> Vec<u8, CATALOG_METADATA_BYTES> {
    let mut output = Vec::new();
    output.extend_from_slice(&METADATA_MAGIC).unwrap();
    output.push(CATALOG_SCHEMA_VERSION).unwrap();
    output
        .push(u8::try_from(metadata.slot).expect("metadata slot is always zero or one"))
        .unwrap();
    output
        .extend_from_slice(&metadata.revision.to_be_bytes())
        .unwrap();
    let checksum = crc32_iso_hdlc(output.as_slice());
    output.extend_from_slice(&checksum.to_be_bytes()).unwrap();
    output
}

fn decode_metadata(bytes: &[u8]) -> Result<Metadata, CatalogDocumentError> {
    if bytes.len() != CATALOG_METADATA_BYTES || bytes[..2] != METADATA_MAGIC {
        return Err(CatalogDocumentError::InvalidMagic);
    }
    if bytes[2] != CATALOG_SCHEMA_VERSION || bytes[3] > 1 {
        return Err(CatalogDocumentError::UnsupportedVersion);
    }
    let expected = u32::from_be_bytes(bytes[12..16].try_into().unwrap());
    if crc32_iso_hdlc(&bytes[..12]) != expected {
        return Err(CatalogDocumentError::InvalidChecksum);
    }
    Ok(Metadata {
        slot: usize::from(bytes[3]),
        revision: u64::from_be_bytes(bytes[4..12].try_into().unwrap()),
    })
}
