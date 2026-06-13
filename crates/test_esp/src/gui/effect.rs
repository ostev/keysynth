use crate::{
    gui::{
        Msg,
        editor::{self, MAX_SIZE, source::Source},
        select_file,
    },
    storage::{self, Files, LoadError, Storage, key_from_name},
    text::{ByteString, Name},
};

pub enum Effect {
    Save(Name, [u8; editor::MAX_SIZE]),
    Load(Name),
    FetchFiles,
}

pub struct Context {
    pub storage: Storage,
}

impl Context {
    async fn save(&mut self, name: Name, bytes: [u8; MAX_SIZE]) -> Result<Name, LoadError> {
        let mut files = self.storage.fetch_files().await?;

        self.storage.save(&key_from_name(&name), &bytes).await?;

        files
            .files
            .insert(name.clone())
            .map_err(|_| LoadError::TooManyFiles)?;

        self.storage
            .save(
                &storage::FILE_LIST_KEY,
                &files.serialize().map_err(LoadError::Serialization)?,
            )
            .await?;

        Ok(name)
    }
}

impl embedded_gui::effect::Effect for Effect {
    type Msg = Msg;
    type Context = Context;

    async fn run(self, context: &mut Context) -> Msg {
        match self {
            Effect::Save(name, bytes) => Msg::SaveCompleted(context.save(name, bytes).await),

            Effect::Load(name) => {
                Msg::LoadCompleted(context.storage.load(&key_from_name(&name)).await.and_then(
                    |serialized: [u8; MAX_SIZE]| {
                        Source::deserialize(name, &serialized).map_err(LoadError::Serialization)
                    },
                ))
            }

            Effect::FetchFiles => Msg::SelectFile(select_file::Msg::FilesReceived(
                context
                    .storage
                    .load(&storage::FILE_LIST_KEY)
                    .await
                    .and_then(|serialized: [u8; MAX_SIZE]| {
                        Files::deserialize(&serialized).map_err(LoadError::Serialization)
                    }),
            )),
        }
    }
}
