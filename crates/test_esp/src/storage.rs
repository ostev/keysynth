mod input {
    use crate::{channel, storage::SaveInstruction};

    channel!(SaveInstruction);
}

mod output {
    use crate::{channel, storage::SaveResult};

    channel!(SaveResult);
}

use core::cell::OnceCell;

use alloc::sync::Arc;
use embassy_embedded_hal::adapter::BlockingAsync;
use embassy_executor::task;
use embassy_sync::{mutex::Mutex, once_lock::OnceLock};
use embedded_storage_async::nor_flash::NorFlash;
use esp_storage::{FlashStorage, FlashStorageError};
use esp_sync::RawMutex;
use heapless::index_set::FnvIndexSet;
pub use input::sender;
pub use output::receiver;
use sequential_storage::{
    cache::PageStateCache,
    map::{MapConfig, MapStorage},
};
use serde::{Deserialize, Serialize};

use crate::text::{ByteChar, FixedByteString, NAME_SIZE, Name};

pub const MAX_SIZE: usize = 2 * 1024;

pub type Key = [ByteChar; NAME_SIZE + 1];

pub const FILE_LIST_KEY: Key = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

pub const MAX_FILES: usize = 40;

pub struct Files {
    pub files: FnvIndexSet<Name, MAX_FILES>,
}

impl Files {
    pub fn new() -> Files {
        Files {
            files: FnvIndexSet::new(),
        }
    }

    pub fn serialize(&self) -> Result<[u8; MAX_SIZE], postcard::Error> {
        let mut bytes = [0; MAX_SIZE];

        postcard::to_slice(&self.files, &mut bytes)?;

        Ok(bytes)
    }

    pub fn deserialize(serialized: &[u8]) -> Result<Files, postcard::Error> {
        Ok(Files {
            files: postcard::from_bytes(serialized)?,
        })
    }
}

pub fn key_from_name(name: &Name) -> Key {
    let mut key: Key = [0; NAME_SIZE + 1];

    for (name_char, key_char) in name.iter().zip(key.iter_mut().skip(1)) {
        *key_char = *name_char;
    }

    key
}

pub struct SaveInstruction {
    name: Key,
    data: [u8; MAX_SIZE],
}

pub type SaveResult = Result<(), ()>;

const MAP_CONFIG: MapConfig<Flash> = MapConfig::new(0xc00000..0xfff000);
const PAGE_CACHE_COUNT: usize = 4;

pub type Flash = BlockingAsync<FlashStorage<'static>>;

#[derive(Debug)]
pub enum LoadError {
    NotFound,
    Flash(sequential_storage::Error<FlashStorageError>),
    Serialization(postcard::Error),
    TooManyFiles,
}

pub struct Storage {
    data_buffer: [u8; MAX_SIZE],
    files: MapStorage<Key, Flash, PageStateCache<PAGE_CACHE_COUNT>>,
}

impl Storage {
    pub fn new(hardware: StorageHardware) -> Storage {
        Storage {
            data_buffer: [0; MAX_SIZE],
            files: MapStorage::new(
                BlockingAsync::new(hardware.flash),
                MAP_CONFIG,
                PageStateCache::<PAGE_CACHE_COUNT>::new(),
            ),
        }
    }

    pub async fn load<const N: usize>(&mut self, key: &Key) -> Result<[u8; N], LoadError> {
        self.files
            .fetch_item(&mut self.data_buffer, key)
            .await
            .map_err(LoadError::Flash)?
            .ok_or(LoadError::NotFound)
    }

    pub async fn save<const N: usize>(
        &mut self,
        key: &Key,
        bytes: &[u8; N],
    ) -> Result<(), LoadError> {
        self.files
            .store_item(&mut self.data_buffer, &key, bytes)
            .await
            .map_err(LoadError::Flash)
    }

    pub async fn delete(&mut self, key: &Key) -> Result<(), LoadError> {
        self.files
            .remove_item(&mut self.data_buffer, key)
            .await
            .map_err(LoadError::Flash)
    }

    pub async fn fetch_files(&mut self) -> Result<Files, LoadError> {
        self.load(&FILE_LIST_KEY)
            .await
            .and_then(|serialized: [u8; MAX_SIZE]| {
                Files::deserialize(&serialized).map_err(LoadError::Serialization)
            })
    }
}

pub struct StorageHardware {
    pub flash: esp_storage::FlashStorage<'static>,
}
