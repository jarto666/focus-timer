use std::{thread, time::Duration};

#[cfg(any(
    feature = "journal-clear-diagnostic",
    feature = "journal-corrupt-diagnostic"
))]
use esp_idf_svc::nvs::EspDefaultNvs;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
#[cfg(feature = "journal-fill-diagnostic")]
use focus_sync::{
    JOURNAL_CAPACITY, PendingRecord, PersistentJournal, PresetSnapshot, SYNC_NAMESPACE,
    SessionOutcome,
};
#[cfg(feature = "journal-fill-diagnostic")]
use heapless::String;

#[cfg(feature = "journal-fill-diagnostic")]
use crate::nvs_journal::{EspEntropy, NvsJournalStore};

/// Performs one explicit mutation in the isolated `focus_sync` namespace and
/// then stays idle. The production `focus_timer` settings namespace is never
/// opened by these diagnostics.
pub(super) fn run() -> ! {
    let partition = EspDefaultNvsPartition::take().expect("default NVS must initialize");

    #[cfg(feature = "journal-clear-diagnostic")]
    {
        let nvs = EspDefaultNvs::new(partition, focus_sync::SYNC_NAMESPACE, true)
            .expect("focus_sync NVS namespace must open");
        nvs.erase_all()
            .expect("focus_sync namespace erase must commit");
        log::warn!(
            "JOURNAL CLEAR DIAGNOSTIC erased only the focus_sync namespace; focus_timer settings were untouched"
        );
    }

    #[cfg(feature = "journal-corrupt-diagnostic")]
    {
        let nvs = EspDefaultNvs::new(partition, focus_sync::SYNC_NAMESPACE, true)
            .expect("focus_sync NVS namespace must open");
        nvs.set_blob("meta_a", b"invalid-meta-a")
            .expect("first corrupt metadata fixture must commit");
        nvs.set_blob("meta_b", b"invalid-meta-b")
            .expect("second corrupt metadata fixture must commit");
        nvs.set_blob("s00", b"invalid-slot")
            .expect("corrupt slot fixture must commit");
        log::warn!(
            "JOURNAL CORRUPT DIAGNOSTIC corrupted redundant metadata and slot s00 only in focus_sync; production must recover or rotate without blocking the timer"
        );
    }

    #[cfg(feature = "journal-fill-diagnostic")]
    {
        let store = NvsJournalStore::open(partition).expect("focus_sync NVS namespace must open");
        let mut entropy = EspEntropy;
        let (mut journal, report) =
            PersistentJournal::<_, JOURNAL_CAPACITY>::initialize(store, &mut entropy)
                .expect("journal must initialize before fill");
        for _ in 0..=JOURNAL_CAPACITY {
            journal
                .append(PendingRecord {
                    preset: PresetSnapshot {
                        id: String::try_from("diagnostic").expect("fixture id must fit"),
                        name: String::try_from("Diagnostic").expect("fixture name must fit"),
                        planned_duration_ms: 1_000,
                    },
                    active_duration_ms: 1_000,
                    outcome: SessionOutcome::Completed,
                    started_at_utc_ms: None,
                    ended_at_utc_ms: None,
                })
                .expect("diagnostic append must commit");
        }
        let epoch = journal.journal().epoch();
        let page = journal
            .page::<8>(Some(epoch), 0, 8)
            .expect("bounded diagnostic page must load");
        log::warn!(
            "JOURNAL FILL DIAGNOSTIC appended {} records in namespace {}; init={report:?} bounds={:?} high_water={} stale_cursor_gap={} first_page_records={}",
            JOURNAL_CAPACITY + 1,
            SYNC_NAMESPACE,
            journal.journal().bounds(),
            journal.journal().high_water_sequence(),
            page.gap,
            page.records.len(),
        );
    }

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
