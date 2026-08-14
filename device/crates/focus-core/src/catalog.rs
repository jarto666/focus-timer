use heapless::{String, Vec};

pub const MAX_PRESET_ID_BYTES: usize = 32;
pub const MAX_PRESET_NAME_BYTES: usize = 32;
pub const BUILT_IN_PRESET_COUNT: usize = 5;
pub const MAX_CUSTOM_PRESETS: usize = 8;
pub const MAX_CATALOG_PRESETS: usize = BUILT_IN_PRESET_COUNT + MAX_CUSTOM_PRESETS;
pub const MIN_CUSTOM_DURATION_MS: u64 = 60_000;
pub const MAX_CUSTOM_DURATION_MS: u64 = 720 * 60_000;

/// Largest preset duration supported by the monotonic representation.
pub const MAX_SESSION_DURATION_MS: u64 = i64::MAX as u64;

const BUILT_IN_IDS: [&str; BUILT_IN_PRESET_COUNT] =
    ["deep-work", "focus", "pomodoro", "reading", "quick-sprint"];

/// Stable bounded identifier persisted across firmware versions.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PresetId(String<MAX_PRESET_ID_BYTES>);

impl PresetId {
    /// Creates an owned identifier. Catalog validation rejects blank values.
    ///
    /// # Panics
    ///
    /// Panics when `value` exceeds [`MAX_PRESET_ID_BYTES`]. External input must use `try_new`.
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self(String::try_from(value).expect("preset identifier must fit the product bound"))
    }

    /// Creates an identifier without panicking when accepting external data.
    ///
    /// # Errors
    ///
    /// Returns [`CatalogError::IdTooLong`] when the UTF-8 byte bound is exceeded.
    pub fn try_new(value: &str) -> Result<Self, CatalogError> {
        String::try_from(value)
            .map(Self)
            .map_err(|()| CatalogError::IdTooLong)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Fixed-capacity owned focus preset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Preset {
    pub id: PresetId,
    pub name: String<MAX_PRESET_NAME_BYTES>,
    pub duration_ms: u64,
    pub built_in: bool,
}

impl Preset {
    /// Legacy/internal constructor. The entry is treated as built-in.
    #[must_use]
    pub fn new(id: &str, name: &str, duration_ms: u64) -> Self {
        Self::built_in(id, name, duration_ms)
    }

    #[must_use]
    /// Creates a trusted built-in preset.
    ///
    /// # Panics
    ///
    /// Panics when firmware-owned identifiers or names exceed their fixed bounds.
    pub fn built_in(id: &str, name: &str, duration_ms: u64) -> Self {
        Self::try_from_parts(id, name, duration_ms, true)
            .expect("built-in preset must satisfy product bounds")
    }

    /// Creates one externally supplied custom preset.
    ///
    /// # Errors
    ///
    /// Returns a catalog bound error for oversized identifiers or names.
    pub fn custom(id: &str, name: &str, duration_ms: u64) -> Result<Self, CatalogError> {
        Self::try_from_parts(id, name, duration_ms, false)
    }

    fn try_from_parts(
        id: &str,
        name: &str,
        duration_ms: u64,
        built_in: bool,
    ) -> Result<Self, CatalogError> {
        Ok(Self {
            id: PresetId::try_new(id)?,
            name: String::try_from(name).map_err(|()| CatalogError::NameTooLong)?,
            duration_ms,
            built_in,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogError {
    Empty,
    CapacityExceeded,
    InvalidDefaultIndex,
    IdTooLong,
    NameTooLong,
    BlankId { index: usize },
    BlankName { index: usize },
    ZeroDuration { index: usize },
    DurationOutOfRange { index: usize },
    CustomDurationNotWholeMinute { index: usize },
    DuplicateId { first: usize, duplicate: usize },
    InvalidBuiltInOrder { index: usize },
    TooManyCustomPresets,
}

/// Validated owned catalog used by the application core.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Catalog {
    presets: Vec<Preset, MAX_CATALOG_PRESETS>,
    default_index: usize,
}

impl Catalog {
    /// Validates generic catalog invariants and clones values into fixed storage.
    ///
    /// # Errors
    ///
    /// Returns a precise [`CatalogError`] when bounds, uniqueness, or the default are invalid.
    pub fn new(presets: &[Preset], default_index: usize) -> Result<Self, CatalogError> {
        validate_common(presets, default_index)?;
        let mut owned = Vec::new();
        for preset in presets {
            owned
                .push(preset.clone())
                .map_err(|_| CatalogError::CapacityExceeded)?;
        }
        Ok(Self {
            presets: owned,
            default_index,
        })
    }

    /// Validates the product catalog: five immutable built-ins, then up to eight customs.
    ///
    /// # Errors
    ///
    /// Returns a precise [`CatalogError`] when the product ordering or custom contract is invalid.
    pub fn combined(presets: &[Preset]) -> Result<Self, CatalogError> {
        validate_common(presets, 2)?;
        if presets.len() < BUILT_IN_PRESET_COUNT {
            return Err(CatalogError::InvalidBuiltInOrder {
                index: presets.len(),
            });
        }
        for (index, expected_id) in BUILT_IN_IDS.iter().enumerate() {
            let preset = &presets[index];
            if !preset.built_in || preset.id.as_str() != *expected_id {
                return Err(CatalogError::InvalidBuiltInOrder { index });
            }
        }
        if presets[BUILT_IN_PRESET_COUNT..]
            .iter()
            .any(|preset| preset.built_in)
        {
            return Err(CatalogError::InvalidBuiltInOrder {
                index: BUILT_IN_PRESET_COUNT,
            });
        }
        if presets.len() - BUILT_IN_PRESET_COUNT > MAX_CUSTOM_PRESETS {
            return Err(CatalogError::TooManyCustomPresets);
        }
        for (offset, preset) in presets[BUILT_IN_PRESET_COUNT..].iter().enumerate() {
            let index = BUILT_IN_PRESET_COUNT + offset;
            if !(MIN_CUSTOM_DURATION_MS..=MAX_CUSTOM_DURATION_MS).contains(&preset.duration_ms) {
                return Err(CatalogError::DurationOutOfRange { index });
            }
            if preset.duration_ms % MIN_CUSTOM_DURATION_MS != 0 {
                return Err(CatalogError::CustomDurationNotWholeMinute { index });
            }
        }
        Self::new(presets, 2)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.presets.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.presets.is_empty()
    }

    #[must_use]
    pub const fn default_index(&self) -> usize {
        self.default_index
    }

    #[must_use]
    pub fn preset(&self, index: usize) -> Preset {
        self.presets[index].clone()
    }

    #[must_use]
    pub fn presets(&self) -> &[Preset] {
        self.presets.as_slice()
    }

    #[must_use]
    pub fn find(&self, id: &str) -> Option<usize> {
        self.presets
            .iter()
            .position(|preset| preset.id.as_str() == id)
    }
}

/// Builds the immutable built-ins in visible encoder order.
#[must_use]
/// Returns the trusted five-entry firmware catalog.
///
/// # Panics
///
/// Panics only if firmware-owned constants stop satisfying their declared capacities.
pub fn default_presets() -> Vec<Preset, MAX_CATALOG_PRESETS> {
    let mut presets = Vec::new();
    for preset in [
        Preset::built_in("deep-work", "Deep Work", 90 * 60_000),
        Preset::built_in("focus", "Focus", 50 * 60_000),
        Preset::built_in("pomodoro", "Pomodoro", 25 * 60_000),
        Preset::built_in("reading", "Reading", 45 * 60_000),
        Preset::built_in("quick-sprint", "Quick Sprint", 15 * 60_000),
    ] {
        presets.push(preset).expect("built-in catalog must fit");
    }
    presets
}

#[must_use]
/// Returns the validated default product catalog.
///
/// # Panics
///
/// Panics only if firmware-owned default preset constants become invalid.
pub fn default_catalog() -> Catalog {
    Catalog::combined(default_presets().as_slice()).expect("built-in catalog must be valid")
}

fn validate_common(presets: &[Preset], default_index: usize) -> Result<(), CatalogError> {
    if presets.is_empty() {
        return Err(CatalogError::Empty);
    }
    if presets.len() > MAX_CATALOG_PRESETS {
        return Err(CatalogError::CapacityExceeded);
    }
    if default_index >= presets.len() {
        return Err(CatalogError::InvalidDefaultIndex);
    }
    for (index, preset) in presets.iter().enumerate() {
        if is_blank(preset.id.as_str()) {
            return Err(CatalogError::BlankId { index });
        }
        if is_blank(preset.name.as_str()) {
            return Err(CatalogError::BlankName { index });
        }
        if preset.duration_ms == 0 {
            return Err(CatalogError::ZeroDuration { index });
        }
        if preset.duration_ms > MAX_SESSION_DURATION_MS {
            return Err(CatalogError::DurationOutOfRange { index });
        }
        if let Some(first) = presets[..index]
            .iter()
            .position(|candidate| candidate.id == preset.id)
        {
            return Err(CatalogError::DuplicateId {
                first,
                duplicate: index,
            });
        }
    }
    Ok(())
}

fn is_blank(value: &str) -> bool {
    value.is_empty() || value.bytes().all(|byte| byte.is_ascii_whitespace())
}
