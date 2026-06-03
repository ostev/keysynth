use crate::{
    gui::{
        Msg,
        editor::{self, MAX_SIZE, source::Source},
    },
    storage::{self, Name},
    text::ByteString,
};

pub enum Effect {
    Save(Name, [u8; editor::MAX_SIZE]),
    Load(Name),
}

impl embedded_gui::effect::Effect for Effect {
    type Msg = Msg;

    async fn run(self) -> Msg {
        match self {
            Effect::Save(name, bytes) => {
                Msg::SaveCompleted(storage::save(&name, &bytes).await.map(move |_| name))
            }
            Effect::Load(name) => Msg::LoadCompleted(
                storage::load(&name)
                    .await
                    .map_err(LoadError::Storage)
                    .and_then(|serialized: [u8; MAX_SIZE]| {
                        Source::deserialize(name, &serialized).map_err(LoadError::Deserialization)
                    }),
            ),
        }
    }
}

#[derive(Debug)]
pub enum LoadError {
    Storage(storage::LoadError),
    Deserialization(postcard::Error),
}
