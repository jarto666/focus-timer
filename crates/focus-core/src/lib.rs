#![no_std]

//! Platform-independent focus timer behavior.
//!
//! This crate deliberately owns no GPIO, display, storage, clock, or logging
//! implementation. Firmware adapters translate hardware activity into semantic
//! events and execute the best-effort effects returned by [`App::handle`].

mod app;
mod catalog;
mod settings;

pub use app::{
    App, AppSnapshot, Diagnostic, Effects, FeedbackPattern, InputEvent, SessionState, ViewState,
};
pub use catalog::{
    Catalog, CatalogError, DEFAULT_PRESETS, MAX_SESSION_DURATION_MS, Preset, PresetId,
};
pub use settings::{
    SETTINGS_SCHEMA_VERSION, SettingsFallback, SettingsLoad, SettingsRecord, restore_selection,
};
