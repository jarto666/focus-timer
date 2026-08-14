#![no_std]

//! Platform-independent parts of the ESP32-C3 firmware adapters.
//!
//! Keeping raw-input classification free of ESP-IDF calls lets the exact same
//! decoder run in host tests and in the device polling loop.

pub mod buzzer;
pub mod input;
pub mod journal_adapter;
pub mod presentation;
pub mod settings;
