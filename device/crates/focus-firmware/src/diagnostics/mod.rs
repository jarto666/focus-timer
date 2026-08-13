//! Complete alternative firmware entrypoints selected by Cargo features.

#[cfg(not(any(
    feature = "ring-diagnostic",
    feature = "encoder-diagnostic",
    feature = "oled-diagnostic",
    feature = "buzzer-diagnostic",
    feature = "settings-clear-diagnostic",
    feature = "settings-corrupt-diagnostic"
)))]
compile_error!(
    "`diagnostic-firmware` is an internal aggregate; select one concrete diagnostic feature"
);

#[cfg(feature = "buzzer-diagnostic")]
mod buzzer;
#[cfg(feature = "encoder-diagnostic")]
mod encoder;
#[cfg(feature = "oled-diagnostic")]
mod oled;
#[cfg(feature = "ring-diagnostic")]
mod ring;
#[cfg(any(
    feature = "settings-clear-diagnostic",
    feature = "settings-corrupt-diagnostic"
))]
mod settings;

#[cfg(any(
    all(feature = "ring-diagnostic", feature = "encoder-diagnostic"),
    all(feature = "ring-diagnostic", feature = "oled-diagnostic"),
    all(feature = "ring-diagnostic", feature = "buzzer-diagnostic"),
    all(feature = "encoder-diagnostic", feature = "oled-diagnostic"),
    all(feature = "encoder-diagnostic", feature = "buzzer-diagnostic"),
    all(feature = "oled-diagnostic", feature = "buzzer-diagnostic"),
    all(feature = "ring-diagnostic", feature = "settings-clear-diagnostic"),
    all(feature = "ring-diagnostic", feature = "settings-corrupt-diagnostic"),
    all(feature = "encoder-diagnostic", feature = "settings-clear-diagnostic"),
    all(
        feature = "encoder-diagnostic",
        feature = "settings-corrupt-diagnostic"
    ),
    all(feature = "oled-diagnostic", feature = "settings-clear-diagnostic"),
    all(feature = "oled-diagnostic", feature = "settings-corrupt-diagnostic"),
    all(feature = "buzzer-diagnostic", feature = "settings-clear-diagnostic"),
    all(feature = "buzzer-diagnostic", feature = "settings-corrupt-diagnostic"),
    all(
        feature = "settings-clear-diagnostic",
        feature = "settings-corrupt-diagnostic"
    )
))]
compile_error!("select exactly one hardware diagnostic feature");

pub(super) fn run() {
    #[cfg(feature = "ring-diagnostic")]
    ring::run();

    #[cfg(feature = "encoder-diagnostic")]
    encoder::run();

    #[cfg(feature = "oled-diagnostic")]
    oled::run();

    #[cfg(feature = "buzzer-diagnostic")]
    buzzer::run();

    #[cfg(any(
        feature = "settings-clear-diagnostic",
        feature = "settings-corrupt-diagnostic"
    ))]
    settings::run();
}
