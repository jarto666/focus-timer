use focus_sync::{
    IDENTITY_RECORD_BYTES, MAX_ENCODED_JOURNAL_RECORD_BYTES, MAX_JOURNAL_SEQUENCE,
    MAX_PRESET_ID_BYTES, MAX_PRESET_NAME_BYTES, METADATA_RECORD_BYTES, MetadataRecord,
    PresetSnapshot, SessionOutcome, StorageDecodeError, decode_identity, decode_metadata,
    decode_record, encode_identity, encode_metadata, encode_record,
};
use heapless::String;

const DEVICE_ID: [u8; 16] = [0x11; 16];
const EPOCH: [u8; 8] = [0x22; 8];

fn text<const N: usize>(value: &str) -> String<N> {
    String::try_from(value).unwrap()
}

fn record() -> focus_sync::JournalRecord {
    focus_sync::JournalRecord {
        sequence: 7,
        preset: PresetSnapshot {
            id: text("pomodoro"),
            name: text("Pomodoro"),
            planned_duration_ms: 1_500_000,
        },
        active_duration_ms: 321_000,
        outcome: SessionOutcome::Cancelled,
        started_at_utc_ms: Some(1_700_000_000_000),
        ended_at_utc_ms: None,
    }
}

#[test]
fn identity_and_metadata_are_versioned_checksummed_round_trips() {
    let identity = encode_identity(DEVICE_ID);
    assert_eq!(identity.len(), IDENTITY_RECORD_BYTES);
    assert_eq!(decode_identity(&identity).unwrap(), DEVICE_ID);

    let metadata = MetadataRecord {
        generation: 42,
        journal_epoch: EPOCH,
        high_water_sequence: 99,
    };
    let encoded = encode_metadata(metadata).unwrap();
    assert_eq!(encoded.len(), METADATA_RECORD_BYTES);
    assert_eq!(decode_metadata(&encoded).unwrap(), metadata);
}

#[test]
fn maximum_record_size_is_measured_and_round_trips() {
    let id_source = "i".repeat(MAX_PRESET_ID_BYTES);
    let name_source = "n".repeat(MAX_PRESET_NAME_BYTES);
    let record = focus_sync::JournalRecord {
        sequence: MAX_JOURNAL_SEQUENCE,
        preset: PresetSnapshot {
            id: String::try_from(id_source.as_str()).unwrap(),
            name: String::try_from(name_source.as_str()).unwrap(),
            planned_duration_ms: u64::MAX,
        },
        active_duration_ms: u64::MAX,
        outcome: SessionOutcome::Cancelled,
        started_at_utc_ms: Some(MAX_JOURNAL_SEQUENCE),
        ended_at_utc_ms: Some(MAX_JOURNAL_SEQUENCE),
    };
    let encoded = encode_record(EPOCH, &record).unwrap();
    assert_eq!(encoded.len(), MAX_ENCODED_JOURNAL_RECORD_BYTES);
    let decoded = decode_record(encoded.as_slice()).unwrap();
    assert_eq!(decoded.journal_epoch, EPOCH);
    assert_eq!(decoded.record, record);
}

#[test]
fn optional_timestamps_and_outcomes_round_trip() {
    let cancelled = record();
    let encoded = encode_record(EPOCH, &cancelled).unwrap();
    assert_eq!(decode_record(encoded.as_slice()).unwrap().record, cancelled);

    let mut completed = cancelled;
    completed.outcome = SessionOutcome::Completed;
    completed.started_at_utc_ms = None;
    completed.ended_at_utc_ms = Some(1_700_000_001_000);
    let encoded = encode_record(EPOCH, &completed).unwrap();
    assert_eq!(decode_record(encoded.as_slice()).unwrap().record, completed);
}

#[test]
fn corruption_and_unsupported_versions_are_rejected() {
    let mut encoded = encode_record(EPOCH, &record()).unwrap();
    encoded[20] ^= 0x80;
    assert_eq!(
        decode_record(encoded.as_slice()).unwrap_err(),
        StorageDecodeError::ChecksumMismatch
    );

    let mut identity = encode_identity(DEVICE_ID);
    identity[2] = 2;
    assert_eq!(
        decode_identity(&identity).unwrap_err(),
        StorageDecodeError::UnsupportedVersion
    );
}
