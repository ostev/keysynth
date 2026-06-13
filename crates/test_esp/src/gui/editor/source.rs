use core::ops::Index;

use alloc::vec;
use alloc::vec::Vec;
use embedded_storage_async::nor_flash::NorFlash;
use sequential_storage::{cache::KeyCacheImpl, map::MapStorage};
use serde::{Deserialize, Serialize};

use crate::{
    gui::editor::{
        history::Edit,
        position::{Position, SelectionRange},
    },
    text::{ByteChar, ByteStr, ByteString, Name},
};

/// Each program can be a maximum of 2KB
pub const MAX_SIZE: usize = 2 * 1024;

pub struct Source {
    pub name: Name,
    // TODO: replace with a gap buffer for better insert performance
    pub lines: Vec<ByteString>,
}

// impl Default for Source {
//     fn default() -> Self {
//         Self {
//             name: None,
//             lines: vec![ByteString::new()],
//         }
//     }
// }

impl Source {
    pub fn new(name: Name) -> Self {
        Self {
            name,
            lines: vec![ByteString::new()],
        }
    }

    pub fn insert(&mut self, character: ByteChar, position: Position) -> Position {
        match character {
            b'\n' => {
                self.lines.insert(position.line, ByteString::new());

                position.advance_newline()
            }
            _ => {
                self.lines[position.line].insert(position.column, character);

                position.advance()
            }
        }
    }

    pub fn delete(&mut self, character: ByteChar, position: Position) -> Position {
        match character {
            b'\n' => {
                self.lines.remove(position.line);

                position.retreat_newline()
            }
            _ => {
                self.lines[position.line].remove(position.column);

                position.retreat()
            }
        }
    }

    pub fn group_insert(&mut self, text: &ByteStr, start: Position) -> Position {
        text.iter().fold(start, |position, character| {
            self.insert(*character, position)
        })
    }

    pub fn group_delete(&mut self, text: &ByteStr, start: Position) -> Position {
        text.iter().fold(start, |position, character| {
            self.delete(*character, position)
        })
    }

    pub fn apply(&mut self, edit: Edit) -> Position {
        match edit {
            Edit::Insert(character, position) => self.insert(character, position),
            Edit::Delete(character, position) => self.delete(character, position),

            Edit::GroupInsert(text, start) => self.group_insert(&text, start),
            Edit::GroupDelete(text, start) => self.group_delete(&text, start),
        }
    }

    // pub async fn load_from_flash<S: NorFlash>(
    //     name: Name,
    //     storage: &mut MapStorage<Name, S, impl KeyCacheImpl<Name>>,
    // ) -> Result<Source, LoadError<S>> {
    //     let mut data_buffer = [0; MAX_SIZE];

    //     let serialized = storage
    //         .fetch_item::<&[u8]>(&mut data_buffer, &name)
    //         .await
    //         .map_err(LoadError::Storage)?
    //         .ok_or(LoadError::NotFound)?;
    //     let source: SerializedSource =
    //         postcard::from_bytes(serialized).map_err(LoadError::DeserializationError)?;

    //     Ok(Source::from_serialized(name, source))
    // }

    pub fn serialize(&self) -> Result<[u8; MAX_SIZE], postcard::Error> {
        let mut bytes = [0; MAX_SIZE];

        postcard::to_slice(&SerializedSource::from(self), &mut bytes)?;

        Ok(bytes)
    }

    pub fn deserialize(name: Name, serialized: &[u8]) -> Result<Source, postcard::Error> {
        let source: SerializedSource = postcard::from_bytes(serialized)?;

        Ok(Source::from_serialized(name, source))
    }

    // pub async fn save_to_flash<S: NorFlash>(
    //     &self,
    //     storage: &mut MapStorage<Name, S, impl KeyCacheImpl<Name>>,
    // ) -> Result<(), SaveError<S>> {
    //     let mut data_buffer = [0; MAX_SIZE];

    //     let mut bytes = [0; MAX_SIZE];

    //     postcard::to_slice(&SerializedSource::from(self), &mut bytes)
    //         .map_err(SaveError::SerializationError)?;

    //     storage.store_item(&mut data_buffer, &self.name, &mut bytes);

    //     Ok(())
    // }

    fn from_serialized(name: Name, serialized: SerializedSource) -> Source {
        Source {
            name,
            lines: serialized
                .text
                .split(|&character| character == b'\n')
                .map(ByteString::from)
                .collect(),
        }
    }

    pub fn get_range(&self, range: SelectionRange) -> ByteString {
        let mut text = ByteString::new();
        text.extend_from_slice(&self.lines[range.start.line][range.start.column..]);

        for line in self.lines[range.start.line + 1..range.end.line].iter() {
            text.extend_from_slice(line);
        }

        text.extend_from_slice(&self.lines[range.end.line][..range.end.column]);

        text
    }

    pub fn to_byte_string(&self) -> ByteString {
        SerializedSource::from(self).text
    }

    pub fn to_string(&self) -> Result<alloc::string::String, alloc::string::FromUtf8Error> {
        alloc::string::String::from_utf8(self.to_byte_string())
    }

    pub unsafe fn to_string_unchecked(&self) -> alloc::string::String {
        unsafe { alloc::string::String::from_utf8_unchecked(self.to_byte_string()) }
    }
}

impl Index<Position> for Source {
    type Output = ByteChar;

    fn index(&self, position: Position) -> &Self::Output {
        &self.lines[position.line][position.column]
    }
}

pub enum LoadError<S: NorFlash> {
    Storage(sequential_storage::Error<S::Error>),
    DeserializationError(postcard::Error),
    NotFound,
}

pub enum SaveError<S: NorFlash> {
    Storage(sequential_storage::Error<S::Error>),
    SerializationError(postcard::Error),
}

#[derive(Deserialize, Serialize)]
struct SerializedSource {
    text: ByteString,
}

impl From<&Source> for SerializedSource {
    fn from(source: &Source) -> Self {
        let text = source
            .lines
            .iter()
            .fold(ByteString::new(), |mut text: ByteString, line| {
                text.push(b'\n');
                text.extend_from_slice(line);
                text
            });

        Self { text }
    }
}
