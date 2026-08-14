use esp_idf_svc::{
    nvs::{EspDefaultNvs, EspDefaultNvsPartition},
    sys::EspError,
};
use focus_sync::{BlobStore, EntropySource, SYNC_NAMESPACE};

pub enum NvsJournalError {
    Esp(EspError),
    RecordTooLarge { length: usize },
}

impl core::fmt::Debug for NvsJournalError {
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

pub struct NvsJournalStore {
    nvs: EspDefaultNvs,
}

impl NvsJournalStore {
    pub fn open(partition: EspDefaultNvsPartition) -> Result<Self, EspError> {
        let nvs = EspDefaultNvs::new(partition, SYNC_NAMESPACE, true)?;
        Ok(Self { nvs })
    }
}

impl BlobStore for NvsJournalStore {
    type Error = NvsJournalError;

    fn read(&mut self, key: &str, output: &mut [u8]) -> Result<Option<usize>, Self::Error> {
        let Some(length) = self.nvs.blob_len(key).map_err(NvsJournalError::Esp)? else {
            return Ok(None);
        };
        if length > output.len() {
            return Err(NvsJournalError::RecordTooLarge { length });
        }
        self.nvs
            .get_blob(key, output)
            .map(|record| record.map(<[u8]>::len))
            .map_err(NvsJournalError::Esp)
    }

    fn write(&mut self, key: &str, bytes: &[u8]) -> Result<(), Self::Error> {
        self.nvs.set_blob(key, bytes).map_err(NvsJournalError::Esp)
    }
}

pub struct EspEntropy;

impl EntropySource for EspEntropy {
    type Error = getrandom::Error;

    fn fill(&mut self, output: &mut [u8]) -> Result<(), Self::Error> {
        getrandom::fill(output)
    }
}
