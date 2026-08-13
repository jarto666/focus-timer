use std::{thread, time::Duration};

use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition};

const NAMESPACE: &str = "focus_timer";
const RECORD_KEY: &str = "settings";

/// Performs one explicit NVS fallback-test mutation and then stays idle.
pub(super) fn run() -> ! {
    let partition = EspDefaultNvsPartition::take().expect("default NVS must initialize");
    let nvs = EspDefaultNvs::new(partition, NAMESPACE, true)
        .expect("focus_timer NVS namespace must open");

    #[cfg(feature = "settings-clear-diagnostic")]
    {
        let existed = nvs
            .remove(RECORD_KEY)
            .expect("settings key removal must commit");
        log::warn!("SETTINGS CLEAR DIAGNOSTIC complete; previous record existed={existed}");
    }

    #[cfg(feature = "settings-corrupt-diagnostic")]
    {
        nvs.set_blob(RECORD_KEY, b"intentionally-corrupt")
            .expect("corrupt settings fixture must commit");
        log::warn!("SETTINGS CORRUPT DIAGNOSTIC wrote an intentionally invalid record");
    }

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
