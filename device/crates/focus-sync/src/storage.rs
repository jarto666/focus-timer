use heapless::{String, Vec};

use crate::{
    DeviceId, JournalEpoch, JournalRecord, MAX_JOURNAL_SEQUENCE, MAX_PRESET_ID_BYTES,
    MAX_PRESET_NAME_BYTES, PresetSnapshot, SessionOutcome,
};

pub const JOURNAL_CAPACITY: usize = 64;
pub const IDENTITY_RECORD_BYTES: usize = 23;
pub const METADATA_RECORD_BYTES: usize = 27;
pub const MAX_ENCODED_JOURNAL_RECORD_BYTES: usize = 122;

const SCHEMA_VERSION: u8 = 1;
const IDENTITY_MAGIC: [u8; 2] = *b"FI";
const METADATA_MAGIC: [u8; 2] = *b"FM";
const RECORD_MAGIC: [u8; 2] = *b"FR";
const FLAG_CANCELLED: u8 = 1 << 0;
const FLAG_STARTED_AT: u8 = 1 << 1;
const FLAG_ENDED_AT: u8 = 1 << 2;
const KNOWN_FLAGS: u8 = FLAG_CANCELLED | FLAG_STARTED_AT | FLAG_ENDED_AT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MetadataRecord {
    pub generation: u32,
    pub journal_epoch: JournalEpoch,
    pub high_water_sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredJournalRecord {
    pub journal_epoch: JournalEpoch,
    pub record: JournalRecord,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageEncodeError {
    InvalidSequence,
    InvalidPreset,
    InvalidDuration,
    TimestampOutOfRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageDecodeError {
    BadLength,
    BadMagic,
    UnsupportedVersion,
    ChecksumMismatch,
    UnknownFlags,
    InvalidSequence,
    InvalidPreset,
    InvalidDuration,
    InvalidUtf8,
    TimestampOutOfRange,
}

#[must_use]
pub fn encode_identity(device_id: DeviceId) -> [u8; IDENTITY_RECORD_BYTES] {
    let mut output = [0_u8; IDENTITY_RECORD_BYTES];
    output[..2].copy_from_slice(&IDENTITY_MAGIC);
    output[2] = SCHEMA_VERSION;
    output[3..19].copy_from_slice(&device_id);
    write_checksum(&mut output);
    output
}

/// Decodes a versioned checksummed stable identity.
///
/// # Errors
///
/// Rejects wrong length, magic, version, or checksum.
pub fn decode_identity(bytes: &[u8]) -> Result<DeviceId, StorageDecodeError> {
    validate_record_header(bytes, IDENTITY_RECORD_BYTES, IDENTITY_MAGIC)?;
    let mut device_id = [0_u8; 16];
    device_id.copy_from_slice(&bytes[3..19]);
    Ok(device_id)
}

/// Encodes one redundant metadata copy.
///
/// # Errors
///
/// Rejects a high-water sequence outside the journal contract.
pub fn encode_metadata(
    metadata: MetadataRecord,
) -> Result<[u8; METADATA_RECORD_BYTES], StorageEncodeError> {
    if metadata.high_water_sequence > MAX_JOURNAL_SEQUENCE {
        return Err(StorageEncodeError::InvalidSequence);
    }
    let mut output = [0_u8; METADATA_RECORD_BYTES];
    output[..2].copy_from_slice(&METADATA_MAGIC);
    output[2] = SCHEMA_VERSION;
    output[3..7].copy_from_slice(&metadata.generation.to_be_bytes());
    output[7..15].copy_from_slice(&metadata.journal_epoch);
    output[15..23].copy_from_slice(&metadata.high_water_sequence.to_be_bytes());
    write_checksum(&mut output);
    Ok(output)
}

/// Decodes one redundant metadata copy.
///
/// # Errors
///
/// Rejects invalid framing, checksum, or high-water sequence.
pub fn decode_metadata(bytes: &[u8]) -> Result<MetadataRecord, StorageDecodeError> {
    validate_record_header(bytes, METADATA_RECORD_BYTES, METADATA_MAGIC)?;
    let generation = read_u32(bytes, 3)?;
    let mut journal_epoch = [0_u8; 8];
    journal_epoch.copy_from_slice(&bytes[7..15]);
    let high_water_sequence = read_u64(bytes, 15)?;
    if high_water_sequence > MAX_JOURNAL_SEQUENCE {
        return Err(StorageDecodeError::InvalidSequence);
    }
    Ok(MetadataRecord {
        generation,
        journal_epoch,
        high_water_sequence,
    })
}

/// Encodes a fixed-schema slot while retaining variable-length preset text.
///
/// # Errors
///
/// Rejects invalid sequences, empty presets, impossible durations, and
/// timestamps outside the protocol-safe integer range.
pub fn encode_record(
    journal_epoch: JournalEpoch,
    record: &JournalRecord,
) -> Result<Vec<u8, MAX_ENCODED_JOURNAL_RECORD_BYTES>, StorageEncodeError> {
    validate_model(record)?;

    let mut output = Vec::new();
    push_bytes(&mut output, &RECORD_MAGIC);
    push(&mut output, SCHEMA_VERSION);
    let mut flags = match record.outcome {
        SessionOutcome::Completed => 0,
        SessionOutcome::Cancelled => FLAG_CANCELLED,
    };
    if record.started_at_utc_ms.is_some() {
        flags |= FLAG_STARTED_AT;
    }
    if record.ended_at_utc_ms.is_some() {
        flags |= FLAG_ENDED_AT;
    }
    push(&mut output, flags);
    push_bytes(&mut output, &journal_epoch);
    push_bytes(&mut output, &record.sequence.to_be_bytes());
    push_bytes(
        &mut output,
        &record.preset.planned_duration_ms.to_be_bytes(),
    );
    push_bytes(&mut output, &record.active_duration_ms.to_be_bytes());
    push_bytes(
        &mut output,
        &record.started_at_utc_ms.unwrap_or_default().to_be_bytes(),
    );
    push_bytes(
        &mut output,
        &record.ended_at_utc_ms.unwrap_or_default().to_be_bytes(),
    );
    let id_length =
        u8::try_from(record.preset.id.len()).map_err(|_| StorageEncodeError::InvalidPreset)?;
    let name_length =
        u8::try_from(record.preset.name.len()).map_err(|_| StorageEncodeError::InvalidPreset)?;
    push(&mut output, id_length);
    push(&mut output, name_length);
    push_bytes(&mut output, record.preset.id.as_bytes());
    push_bytes(&mut output, record.preset.name.as_bytes());
    let checksum = crc32_iso_hdlc(output.as_slice());
    push_bytes(&mut output, &checksum.to_be_bytes());
    Ok(output)
}

/// Decodes and validates a journal slot.
///
/// # Errors
///
/// Rejects malformed framing, checksum, flags, text, sequence, duration, or
/// timestamp data without returning a partial record.
pub fn decode_record(bytes: &[u8]) -> Result<StoredJournalRecord, StorageDecodeError> {
    const FIXED_BYTES: usize = 58;
    if bytes.len() < FIXED_BYTES || bytes.len() > MAX_ENCODED_JOURNAL_RECORD_BYTES {
        return Err(StorageDecodeError::BadLength);
    }
    if bytes[..2] != RECORD_MAGIC {
        return Err(StorageDecodeError::BadMagic);
    }
    if bytes[2] != SCHEMA_VERSION {
        return Err(StorageDecodeError::UnsupportedVersion);
    }
    validate_checksum(bytes)?;

    let flags = bytes[3];
    if flags & !KNOWN_FLAGS != 0 {
        return Err(StorageDecodeError::UnknownFlags);
    }
    let mut journal_epoch = [0_u8; 8];
    journal_epoch.copy_from_slice(&bytes[4..12]);
    let sequence = read_u64(bytes, 12)?;
    if sequence == 0 || sequence > MAX_JOURNAL_SEQUENCE {
        return Err(StorageDecodeError::InvalidSequence);
    }
    let planned_duration_ms = read_u64(bytes, 20)?;
    let active_duration_ms = read_u64(bytes, 28)?;
    if planned_duration_ms == 0 || active_duration_ms > planned_duration_ms {
        return Err(StorageDecodeError::InvalidDuration);
    }
    let started_raw = read_u64(bytes, 36)?;
    let ended_raw = read_u64(bytes, 44)?;
    let started_at_utc_ms = decode_timestamp(flags, FLAG_STARTED_AT, started_raw)?;
    let ended_at_utc_ms = decode_timestamp(flags, FLAG_ENDED_AT, ended_raw)?;
    let id_length = usize::from(bytes[52]);
    let name_length = usize::from(bytes[53]);
    let text_end = 54_usize
        .checked_add(id_length)
        .and_then(|end| end.checked_add(name_length))
        .ok_or(StorageDecodeError::BadLength)?;
    if id_length == 0
        || name_length == 0
        || id_length > MAX_PRESET_ID_BYTES
        || name_length > MAX_PRESET_NAME_BYTES
        || text_end + 4 != bytes.len()
    {
        return Err(StorageDecodeError::InvalidPreset);
    }
    let id = core::str::from_utf8(&bytes[54..54 + id_length])
        .map_err(|_| StorageDecodeError::InvalidUtf8)?;
    let name = core::str::from_utf8(&bytes[54 + id_length..text_end])
        .map_err(|_| StorageDecodeError::InvalidUtf8)?;
    let id = String::try_from(id).map_err(|()| StorageDecodeError::InvalidPreset)?;
    let name = String::try_from(name).map_err(|()| StorageDecodeError::InvalidPreset)?;

    Ok(StoredJournalRecord {
        journal_epoch,
        record: JournalRecord {
            sequence,
            preset: PresetSnapshot {
                id,
                name,
                planned_duration_ms,
            },
            active_duration_ms,
            outcome: if flags & FLAG_CANCELLED == 0 {
                SessionOutcome::Completed
            } else {
                SessionOutcome::Cancelled
            },
            started_at_utc_ms,
            ended_at_utc_ms,
        },
    })
}

fn validate_model(record: &JournalRecord) -> Result<(), StorageEncodeError> {
    if record.sequence == 0 || record.sequence > MAX_JOURNAL_SEQUENCE {
        return Err(StorageEncodeError::InvalidSequence);
    }
    if record.preset.id.is_empty() || record.preset.name.is_empty() {
        return Err(StorageEncodeError::InvalidPreset);
    }
    if record.preset.planned_duration_ms == 0
        || record.active_duration_ms > record.preset.planned_duration_ms
    {
        return Err(StorageEncodeError::InvalidDuration);
    }
    if record
        .started_at_utc_ms
        .into_iter()
        .chain(record.ended_at_utc_ms)
        .any(|timestamp| timestamp > MAX_JOURNAL_SEQUENCE)
    {
        return Err(StorageEncodeError::TimestampOutOfRange);
    }
    Ok(())
}

fn decode_timestamp(
    flags: u8,
    presence_flag: u8,
    value: u64,
) -> Result<Option<u64>, StorageDecodeError> {
    if flags & presence_flag == 0 {
        return (value == 0)
            .then_some(None)
            .ok_or(StorageDecodeError::TimestampOutOfRange);
    }
    (value <= MAX_JOURNAL_SEQUENCE)
        .then_some(Some(value))
        .ok_or(StorageDecodeError::TimestampOutOfRange)
}

fn validate_record_header(
    bytes: &[u8],
    expected_length: usize,
    magic: [u8; 2],
) -> Result<(), StorageDecodeError> {
    if bytes.len() != expected_length {
        return Err(StorageDecodeError::BadLength);
    }
    if bytes[..2] != magic {
        return Err(StorageDecodeError::BadMagic);
    }
    if bytes[2] != SCHEMA_VERSION {
        return Err(StorageDecodeError::UnsupportedVersion);
    }
    validate_checksum(bytes)
}

fn validate_checksum(bytes: &[u8]) -> Result<(), StorageDecodeError> {
    let payload_length = bytes
        .len()
        .checked_sub(4)
        .ok_or(StorageDecodeError::BadLength)?;
    let expected = u32::from_be_bytes(bytes[payload_length..].try_into().unwrap());
    if crc32_iso_hdlc(&bytes[..payload_length]) != expected {
        return Err(StorageDecodeError::ChecksumMismatch);
    }
    Ok(())
}

fn read_u32(bytes: &[u8], start: usize) -> Result<u32, StorageDecodeError> {
    let value = bytes
        .get(start..start + 4)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(StorageDecodeError::BadLength)?;
    Ok(u32::from_be_bytes(value))
}

fn read_u64(bytes: &[u8], start: usize) -> Result<u64, StorageDecodeError> {
    let value = bytes
        .get(start..start + 8)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(StorageDecodeError::BadLength)?;
    Ok(u64::from_be_bytes(value))
}

fn write_checksum<const N: usize>(output: &mut [u8; N]) {
    let payload_length = N - 4;
    let checksum = crc32_iso_hdlc(&output[..payload_length]);
    output[payload_length..].copy_from_slice(&checksum.to_be_bytes());
}

fn push<const N: usize>(output: &mut Vec<u8, N>, value: u8) {
    output.push(value).expect("encoded size constant is exact");
}

fn push_bytes<const N: usize>(output: &mut Vec<u8, N>, bytes: &[u8]) {
    output
        .extend_from_slice(bytes)
        .expect("encoded size constant is exact");
}

fn crc32_iso_hdlc(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}
