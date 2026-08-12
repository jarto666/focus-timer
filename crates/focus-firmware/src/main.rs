#[cfg(feature = "buzzer-diagnostic")]
mod buzzer_diagnostic;
#[cfg(not(any(
    feature = "ring-diagnostic",
    feature = "encoder-diagnostic",
    feature = "oled-diagnostic",
    feature = "buzzer-diagnostic"
)))]
mod clock;
#[cfg(feature = "encoder-diagnostic")]
mod encoder_diagnostic;
#[cfg(not(any(
    feature = "ring-diagnostic",
    feature = "encoder-diagnostic",
    feature = "oled-diagnostic",
    feature = "buzzer-diagnostic"
)))]
mod nvs_settings;
#[cfg(feature = "oled-diagnostic")]
mod oled_diagnostic;
#[cfg(feature = "ring-diagnostic")]
mod ring_diagnostic;
#[cfg(not(any(
    feature = "ring-diagnostic",
    feature = "encoder-diagnostic",
    feature = "oled-diagnostic",
    feature = "buzzer-diagnostic"
)))]
mod runtime;

#[cfg(any(
    all(feature = "ring-diagnostic", feature = "encoder-diagnostic"),
    all(feature = "ring-diagnostic", feature = "oled-diagnostic"),
    all(feature = "ring-diagnostic", feature = "buzzer-diagnostic"),
    all(feature = "encoder-diagnostic", feature = "oled-diagnostic"),
    all(feature = "encoder-diagnostic", feature = "buzzer-diagnostic"),
    all(feature = "oled-diagnostic", feature = "buzzer-diagnostic")
))]
compile_error!("select exactly one hardware diagnostic feature");

use esp_idf_svc::{hal::reset::ResetReason, log::EspLogger, sys};

fn main() {
    sys::link_patches();
    EspLogger::initialize_default();

    log::info!(
        "focus-firmware {} booting on ESP32-C3; reset_reason={:?}",
        env!("CARGO_PKG_VERSION"),
        ResetReason::get()
    );
    #[cfg(feature = "ring-diagnostic")]
    ring_diagnostic::run();

    #[cfg(feature = "encoder-diagnostic")]
    encoder_diagnostic::run();

    #[cfg(feature = "oled-diagnostic")]
    oled_diagnostic::run();

    #[cfg(feature = "buzzer-diagnostic")]
    buzzer_diagnostic::run();

    #[cfg(not(any(
        feature = "ring-diagnostic",
        feature = "encoder-diagnostic",
        feature = "oled-diagnostic",
        feature = "buzzer-diagnostic"
    )))]
    runtime::run();
}
