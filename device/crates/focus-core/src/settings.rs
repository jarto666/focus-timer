use crate::Catalog;

pub const SETTINGS_SCHEMA_VERSION: u8 = 1;

/// Decoded versioned settings payload. It deliberately has no session fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SettingsRecord<'a> {
    pub schema_version: u8,
    pub selected_preset_id: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsLoad<'a> {
    Missing,
    Corrupt,
    Record(SettingsRecord<'a>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsFallback {
    Corrupt,
    UnsupportedVersion,
    UnknownPreset,
}

/// Resolves persisted settings to a valid catalog index and optional diagnostic.
#[must_use]
pub fn restore_selection(
    catalog: &Catalog,
    load: SettingsLoad<'_>,
) -> (usize, Option<SettingsFallback>) {
    match load {
        SettingsLoad::Missing => (catalog.default_index(), None),
        SettingsLoad::Corrupt => (catalog.default_index(), Some(SettingsFallback::Corrupt)),
        SettingsLoad::Record(record) if record.schema_version != SETTINGS_SCHEMA_VERSION => (
            catalog.default_index(),
            Some(SettingsFallback::UnsupportedVersion),
        ),
        SettingsLoad::Record(record) => catalog.find(record.selected_preset_id).map_or(
            (
                catalog.default_index(),
                Some(SettingsFallback::UnknownPreset),
            ),
            |index| (index, None),
        ),
    }
}
