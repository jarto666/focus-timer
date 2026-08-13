#[cfg(not(any(
    feature = "ring-diagnostic",
    feature = "encoder-diagnostic",
    feature = "oled-diagnostic",
    feature = "buzzer-diagnostic",
    feature = "settings-clear-diagnostic",
    feature = "settings-corrupt-diagnostic"
)))]
mod clock;
#[cfg(any(
    feature = "ring-diagnostic",
    feature = "encoder-diagnostic",
    feature = "oled-diagnostic",
    feature = "buzzer-diagnostic",
    feature = "settings-clear-diagnostic",
    feature = "settings-corrupt-diagnostic"
))]
mod diagnostics;
#[cfg(not(any(
    feature = "ring-diagnostic",
    feature = "encoder-diagnostic",
    feature = "oled-diagnostic",
    feature = "buzzer-diagnostic",
    feature = "settings-clear-diagnostic",
    feature = "settings-corrupt-diagnostic"
)))]
mod nvs_settings;
#[cfg(not(any(
    feature = "ring-diagnostic",
    feature = "encoder-diagnostic",
    feature = "oled-diagnostic",
    feature = "buzzer-diagnostic",
    feature = "settings-clear-diagnostic",
    feature = "settings-corrupt-diagnostic"
)))]
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
    #[cfg(any(
        feature = "ring-diagnostic",
        feature = "encoder-diagnostic",
        feature = "oled-diagnostic",
        feature = "buzzer-diagnostic",
        feature = "settings-clear-diagnostic",
        feature = "settings-corrupt-diagnostic"
    ))]
    diagnostics::run();

    #[cfg(not(any(
        feature = "ring-diagnostic",
        feature = "encoder-diagnostic",
        feature = "oled-diagnostic",
        feature = "buzzer-diagnostic",
        feature = "settings-clear-diagnostic",
        feature = "settings-corrupt-diagnostic"
    )))]
    runtime::run();
}
