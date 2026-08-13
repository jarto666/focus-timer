use esp_idf_svc::{
    nvs::{EspDefaultNvs, EspDefaultNvsPartition},
    sys::EspError,
};
use focus_firmware::settings::SettingsStore;

const NAMESPACE: &str = "focus_timer";
const RECORD_KEY: &str = "settings";

pub enum NvsSettingsError {
    Esp(EspError),
    RecordTooLarge { length: usize },
}

impl core::fmt::Debug for NvsSettingsError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Esp(error) => formatter.debug_tuple("Esp").field(error).finish(),
            Self::RecordTooLarge { length } => formatter
                .debug_struct("RecordTooLarge")
                .field("length", length)
                .finish(),
        }
    }
}

pub struct NvsSettingsStore {
    nvs: EspDefaultNvs,
}

impl NvsSettingsStore {
    pub fn open() -> Result<Self, EspError> {
        let partition = EspDefaultNvsPartition::take()?;
        let nvs = EspDefaultNvs::new(partition, NAMESPACE, true)?;
        Ok(Self { nvs })
    }
}

impl SettingsStore for NvsSettingsStore {
    type Error = NvsSettingsError;

    fn read(&mut self, output: &mut [u8]) -> Result<Option<usize>, Self::Error> {
        let Some(length) = self
            .nvs
            .blob_len(RECORD_KEY)
            .map_err(NvsSettingsError::Esp)?
        else {
            return Ok(None);
        };
        if length > output.len() {
            return Err(NvsSettingsError::RecordTooLarge { length });
        }

        self.nvs
            .get_blob(RECORD_KEY, output)
            .map(|record| record.map(<[u8]>::len))
            .map_err(NvsSettingsError::Esp)
    }

    fn write(&mut self, record: &[u8]) -> Result<(), Self::Error> {
        self.nvs
            .set_blob(RECORD_KEY, record)
            .map_err(NvsSettingsError::Esp)
    }
}
