use embassy_embedded_hal::adapter::BlockingAsync;
use esp_storage::{FlashStorage, FlashStorageError};
use sequential_storage::{
    cache::PageStateCache,
    map::{MapConfig, MapStorage},
};

use crate::text::{ByteChar, NAME_SIZE, Name};

pub const BUFFER_SIZE: usize = 8 * 1024;

pub const MAX_SIZE: usize = 3 * 1024;

pub type Key = [ByteChar; NAME_SIZE + 1];

pub const FILE_LIST_KEY: Key = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

pub const MAX_FILES: usize = 32;

#[derive(Debug)]
pub struct Files {
    files: heapless::Vec<Name, MAX_FILES>,
}

impl Files {
    pub fn new() -> Files {
        Files {
            files: heapless::Vec::new(),
        }
    }

    pub fn create(&mut self, name: Name) -> Result<(), LoadError> {
        if !self.files.contains(&name) {
            self.files.push(name).map_err(|_| LoadError::TooManyFiles)
        } else {
            Ok(())
        }
    }

    pub fn remove(&mut self, name: &Name) {
        self.files.retain(|file_name| file_name != name);
    }

    pub fn sorted_by_most_recent(mut self) -> heapless::Vec<Name, MAX_FILES> {
        self.files.reverse();
        self.files
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
    data_buffer: [u8; BUFFER_SIZE],
    files: MapStorage<Key, Flash, PageStateCache<PAGE_CACHE_COUNT>>,
}

impl Storage {
    pub fn new(hardware: StorageHardware) -> Storage {
        Storage {
            data_buffer: [0; BUFFER_SIZE],
            files: MapStorage::new(
                BlockingAsync::new(hardware.flash.multicore_auto_park()),
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

    pub async fn save(&mut self, key: &Key, bytes: &[u8]) -> Result<(), LoadError> {
        self.files
            .store_item(&mut self.data_buffer, &key, &bytes)
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
        match self.load::<MAX_SIZE>(&FILE_LIST_KEY).await {
            Ok(serialized) => Files::deserialize(&serialized).map_err(LoadError::Serialization),
            Err(err) => match err {
                LoadError::NotFound => Ok(Files::new()),
                _ => Err(err),
            },
        }
    }
}

pub struct StorageHardware {
    pub flash: esp_storage::FlashStorage<'static>,
}
