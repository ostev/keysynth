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
pub use input::sender;
pub use output::receiver;
use sequential_storage::{
    cache::PageStateCache,
    map::{MapConfig, MapStorage},
};

use crate::text::ByteChar;

pub const MAX_SIZE: usize = 2 * 1024;

pub struct SaveInstruction {
    name: Name,
    data: [u8; MAX_SIZE],
}

pub type SaveResult = Result<(), ()>;

/// Names are null-terminated arrays
pub type Name = [ByteChar; 10];

const MAP_CONFIG: MapConfig<Flash> = MapConfig::new(0xc00000..0xfff000);
const PAGE_CACHE_COUNT: usize = 4;

pub type Flash = BlockingAsync<FlashStorage<'static>>;

static STORAGE: OnceLock<Mutex<RawMutex, Storage>> = OnceLock::new();

pub async fn save<const N: usize>(
    name: &Name,
    bytes: &[u8; N],
) -> Result<(), sequential_storage::Error<FlashStorageError>> {
    let mut data_buffer = [0; MAX_SIZE];

    STORAGE
        .get()
        .await
        .lock()
        .await
        .map
        .store_item(&mut data_buffer, name, bytes)
        .await
}

pub async fn load<const N: usize>(name: &Name) -> Result<[u8; N], LoadError> {
    let mut data_buffer = [0; MAX_SIZE];

    STORAGE
        .get()
        .await
        .lock()
        .await
        .map
        .fetch_item(&mut data_buffer, name)
        .await
        .map_err(LoadError::Flash)?
        .ok_or(LoadError::NotFound)
}

#[derive(Debug)]
pub enum LoadError {
    NotFound,
    Flash(sequential_storage::Error<FlashStorageError>),
}

pub struct Storage {
    map: MapStorage<Name, Flash, PageStateCache<PAGE_CACHE_COUNT>>,
}

pub struct StorageHardware {
    pub flash: esp_storage::FlashStorage<'static>,
}

impl StorageHardware {
    pub fn init(self) -> Result<(), Mutex<RawMutex, Storage>> {
        STORAGE.init(Mutex::new(Storage {
            map: MapStorage::new(
                BlockingAsync::new(self.flash),
                MAP_CONFIG,
                PageStateCache::<PAGE_CACHE_COUNT>::new(),
            ),
        }))
    }
}
