#[cfg(not(feature = "diagnostic-firmware"))]
mod clock;
#[cfg(feature = "diagnostic-firmware")]
mod diagnostics;
#[cfg(not(feature = "diagnostic-firmware"))]
mod nvs_settings;
#[cfg(not(feature = "diagnostic-firmware"))]
mod runtime;

use esp_idf_svc::{hal::reset::ResetReason, log::EspLogger, sys};

fn main() {
    sys::link_patches();
    EspLogger::initialize_default();

    log::info!(
        "focus-firmware {} booting on ESP32-C3; reset_reason={:?}",
        env!("CARGO_PKG_VERSION"),
        ResetReason::get()
    );
    #[cfg(feature = "diagnostic-firmware")]
    diagnostics::run();

    #[cfg(not(feature = "diagnostic-firmware"))]
    runtime::run();
}
