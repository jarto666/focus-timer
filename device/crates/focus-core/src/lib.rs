#![no_std]

//! Platform-independent focus timer behavior.
//!
//! This crate deliberately owns no GPIO, display, storage, clock, or logging
//! implementation. Firmware adapters translate hardware activity into semantic
//! events and execute the best-effort effects returned by [`App::handle`].

mod app;
mod catalog;
mod catalog_update;
mod settings;

pub use app::{
    App, AppSnapshot, CatalogReplaceError, Diagnostic, Effects, FeedbackPattern, InputEvent,
    SessionOutcome, SessionOutcomeKind, SessionState, ViewState,
};
pub use catalog::{
    BUILT_IN_PRESET_COUNT, Catalog, CatalogError, MAX_CATALOG_PRESETS, MAX_CUSTOM_DURATION_MS,
    MAX_CUSTOM_PRESETS, MAX_PRESET_ID_BYTES, MAX_PRESET_NAME_BYTES, MAX_SESSION_DURATION_MS,
    MIN_CUSTOM_DURATION_MS, Preset, PresetId, default_catalog, default_presets,
};
pub use catalog_update::{
    CATALOG_CONFIRMATION_TIMEOUT_MS, CatalogCommit, CatalogConfirmationAction, CatalogStageError,
    CatalogStageOutcome, CatalogUpdateCoordinator,
};
pub use settings::{
    SETTINGS_SCHEMA_VERSION, SettingsFallback, SettingsLoad, SettingsRecord, restore_selection,
};
