/// Largest preset duration supported by the monotonic representation.
pub const MAX_SESSION_DURATION_MS: u64 = i64::MAX as u64;

/// Stable identifier persisted across firmware versions.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PresetId(&'static str);

impl PresetId {
    /// Creates an identifier. Catalog validation rejects blank values.
    #[must_use]
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

/// Compile-time focus preset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Preset {
    pub id: PresetId,
    pub name: &'static str,
    pub duration_ms: u64,
}

impl Preset {
    #[must_use]
    pub const fn new(id: &'static str, name: &'static str, duration_ms: u64) -> Self {
        Self {
            id: PresetId::new(id),
            name,
            duration_ms,
        }
    }
}

const MINUTE_MS: u64 = 60_000;

/// Prototype catalog in visible encoder order.
pub const DEFAULT_PRESETS: [Preset; 5] = [
    Preset::new("deep-work", "Deep Work", 90 * MINUTE_MS),
    Preset::new("focus", "Focus", 50 * MINUTE_MS),
    Preset::new("pomodoro", "Pomodoro", 25 * MINUTE_MS),
    Preset::new("reading", "Reading", 45 * MINUTE_MS),
    Preset::new("quick-sprint", "Quick Sprint", 15 * MINUTE_MS),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogError {
    Empty,
    InvalidDefaultIndex,
    BlankId { index: usize },
    BlankName { index: usize },
    ZeroDuration { index: usize },
    DurationOutOfRange { index: usize },
    DuplicateId { first: usize, duplicate: usize },
}

/// Validated non-empty view of a compile-time catalog.
#[derive(Clone, Copy, Debug)]
pub struct Catalog {
    presets: &'static [Preset],
    default_index: usize,
}

impl Catalog {
    /// Validates catalog invariants before the firmware can use a preset.
    ///
    /// # Errors
    ///
    /// Returns [`CatalogError`] when the catalog is empty, its default index is
    /// invalid, or any preset violates identifier, name, duration, or uniqueness
    /// requirements.
    pub fn new(presets: &'static [Preset], default_index: usize) -> Result<Self, CatalogError> {
        if presets.is_empty() {
            return Err(CatalogError::Empty);
        }
        if default_index >= presets.len() {
            return Err(CatalogError::InvalidDefaultIndex);
        }

        for (index, preset) in presets.iter().enumerate() {
            if is_blank(preset.id.as_str()) {
                return Err(CatalogError::BlankId { index });
            }
            if is_blank(preset.name) {
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

        Ok(Self {
            presets,
            default_index,
        })
    }

    #[must_use]
    pub(crate) const fn len(self) -> usize {
        self.presets.len()
    }

    #[must_use]
    pub const fn default_index(self) -> usize {
        self.default_index
    }

    #[must_use]
    pub fn preset(self, index: usize) -> Preset {
        self.presets[index]
    }

    #[must_use]
    pub fn find(self, id: &str) -> Option<usize> {
        self.presets
            .iter()
            .position(|preset| preset.id.as_str() == id)
    }
}

fn is_blank(value: &str) -> bool {
    value.is_empty() || value.bytes().all(|byte| byte.is_ascii_whitespace())
}
