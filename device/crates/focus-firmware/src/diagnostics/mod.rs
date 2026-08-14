//! Complete alternative firmware entrypoints selected by Cargo features.

const DIAGNOSTIC_COUNT: usize = cfg!(feature = "ring-diagnostic") as usize
    + cfg!(feature = "encoder-diagnostic") as usize
    + cfg!(feature = "oled-diagnostic") as usize
    + cfg!(feature = "buzzer-diagnostic") as usize
    + cfg!(feature = "settings-clear-diagnostic") as usize
    + cfg!(feature = "settings-corrupt-diagnostic") as usize
    + cfg!(feature = "journal-clear-diagnostic") as usize
    + cfg!(feature = "journal-corrupt-diagnostic") as usize
    + cfg!(feature = "journal-fill-diagnostic") as usize;

const _: () = assert!(
    DIAGNOSTIC_COUNT == 1,
    "select exactly one concrete diagnostic feature"
);

#[cfg(feature = "buzzer-diagnostic")]
mod buzzer;
#[cfg(feature = "encoder-diagnostic")]
mod encoder;
#[cfg(any(
    feature = "journal-clear-diagnostic",
    feature = "journal-corrupt-diagnostic",
    feature = "journal-fill-diagnostic"
))]
mod journal;
#[cfg(feature = "oled-diagnostic")]
mod oled;
#[cfg(feature = "ring-diagnostic")]
mod ring;
#[cfg(any(
    feature = "settings-clear-diagnostic",
    feature = "settings-corrupt-diagnostic"
))]
mod settings;

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

    #[cfg(any(
        feature = "journal-clear-diagnostic",
        feature = "journal-corrupt-diagnostic",
        feature = "journal-fill-diagnostic"
    ))]
    journal::run();
}
