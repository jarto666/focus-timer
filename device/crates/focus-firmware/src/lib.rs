#![no_std]

//! Platform-independent parts of the ESP32-C3 firmware adapters.
//!
//! Keeping raw-input classification free of ESP-IDF calls lets the exact same
//! decoder run in host tests and in the device polling loop.

pub mod buzzer;
#[cfg(feature = "display-worker")]
pub mod display_worker;
pub mod input;
pub mod journal_adapter;
pub mod presentation;
pub mod preset_storage;
pub mod protocol_projection;
pub mod protocol_session;
pub mod settings;
#[cfg(feature = "esp-idf-runtime")]
pub mod tft_display;
#[cfg(feature = "tft-renderer")]
pub mod tft_render;
