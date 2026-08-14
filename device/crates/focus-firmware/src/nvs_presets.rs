use esp_idf_svc::{
    nvs::{EspDefaultNvs, EspDefaultNvsPartition},
    sys::EspError,
};
use focus_firmware::preset_storage::{PRESET_NAMESPACE, PresetCatalogStore};

const SLOT_KEYS: [&str; 2] = ["slot0", "slot1"];
const META_KEYS: [&str; 2] = ["meta0", "meta1"];

pub enum NvsPresetError {
    Esp(EspError),
    InvalidIndex(usize),
    RecordTooLarge { length: usize },
}

impl core::fmt::Debug for NvsPresetError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Esp(error) => formatter.debug_tuple("Esp").field(error).finish(),
            Self::InvalidIndex(index) => {
                formatter.debug_tuple("InvalidIndex").field(index).finish()
            }
            Self::RecordTooLarge { length } => formatter
                .debug_struct("RecordTooLarge")
                .field("length", length)
                .finish(),
        }
    }
}

pub struct NvsPresetCatalogStore {
    nvs: EspDefaultNvs,
}

impl NvsPresetCatalogStore {
    pub fn open(partition: EspDefaultNvsPartition) -> Result<Self, EspError> {
        Ok(Self {
            nvs: EspDefaultNvs::new(partition, PRESET_NAMESPACE, true)?,
        })
    }

    fn read_key(&mut self, key: &str, output: &mut [u8]) -> Result<Option<usize>, NvsPresetError> {
        let Some(length) = self.nvs.blob_len(key).map_err(NvsPresetError::Esp)? else {
            return Ok(None);
        };
        if length > output.len() {
            return Err(NvsPresetError::RecordTooLarge { length });
        }
        self.nvs
            .get_blob(key, output)
            .map(|record| record.map(<[u8]>::len))
            .map_err(NvsPresetError::Esp)
    }
}

impl PresetCatalogStore for NvsPresetCatalogStore {
    type Error = NvsPresetError;

    fn read_slot(&mut self, slot: usize, output: &mut [u8]) -> Result<Option<usize>, Self::Error> {
        let key = SLOT_KEYS
            .get(slot)
            .ok_or(NvsPresetError::InvalidIndex(slot))?;
        self.read_key(key, output)
    }

    fn write_slot(&mut self, slot: usize, document: &[u8]) -> Result<(), Self::Error> {
        let key = SLOT_KEYS
            .get(slot)
            .ok_or(NvsPresetError::InvalidIndex(slot))?;
        self.nvs
            .set_blob(key, document)
            .map_err(NvsPresetError::Esp)
    }

    fn read_metadata(
        &mut self,
        copy: usize,
        output: &mut [u8],
    ) -> Result<Option<usize>, Self::Error> {
        let key = META_KEYS
            .get(copy)
            .ok_or(NvsPresetError::InvalidIndex(copy))?;
        self.read_key(key, output)
    }

    fn write_metadata(&mut self, copy: usize, metadata: &[u8]) -> Result<(), Self::Error> {
        let key = META_KEYS
            .get(copy)
            .ok_or(NvsPresetError::InvalidIndex(copy))?;
        self.nvs
            .set_blob(key, metadata)
            .map_err(NvsPresetError::Esp)
    }
}
