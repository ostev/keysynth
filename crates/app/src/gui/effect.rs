use core::sync::atomic::Ordering;

use alloc::string::{String, ToString};
use atomic::Atomic;
use bumpalo::Bump;
use codespan_reporting::diagnostic::Diagnostic;
use esp_println::println;
use rpds::List;
use synth::wavetable::Wavetable;

use crate::{
    gui::{
        Msg, PageMsg,
        editor::{self, MAX_SIZE, source::Source},
        select_file,
    },
    input::encoder,
    storage::{self, Files, LoadError, Storage, key_from_name},
    text::{Name, fixed_str_to_str},
    usb,
};

/// The currently selected encoder control panel.
static PANEL: Atomic<encoder::Panel> = Atomic::new(encoder::Panel::CutoffResonance);

/// Get the currently selected encoder control panel.
pub fn current_panel() -> encoder::Panel {
    PANEL.load(Ordering::Relaxed)
}

/// Side effects that may be performed in response to user actions.
///
/// Effects encapsulate asynchronous operations such as interacting with
/// storage or updating global app state.
pub(super) enum Effect {
    /// Compile the source code into a wavetable and send it to the audio
    /// engine.
    BuildWavetable(Name, String),
    /// Save a file to flash.
    Save(Name, [u8; editor::MAX_SIZE]),
    /// Load a file from flash.
    Load(Name),
    /// Delete a file on flash.
    Delete(Name),
    /// Rename a file on flash.
    Rename { old: Name, new: Name },
    /// Fetch the list of files from flash.
    FetchFiles,
    /// Update the active synth parameter control panel.
    SetPanel(encoder::Panel),
}

/// Context provided to effect handlers.
///
/// Owns the resources required to perform asynchronous effects.
pub(super) struct Context {
    /// Flash storage.
    pub storage: Storage,
    /// Arena allocator used to store AST when compiling wavetables.
    pub ast_arena: Bump,
}

impl Context {
    /// Save a file to flash.
    async fn save(&mut self, name: Name, bytes: [u8; MAX_SIZE]) -> Result<Name, LoadError> {
        let mut files = self.storage.fetch_files().await?;

        self.storage
            .save(&key_from_name(&name), &bytes)
            .await
            .unwrap();

        files.create(name.clone())?;

        self.save_files(&files).await?;

        println!("Saved file to flash!");

        Ok(name)
    }

    /// Save the file list metadata.
    async fn save_files(&mut self, files: &Files) -> Result<(), LoadError> {
        self.storage
            .save(
                &storage::FILE_LIST_KEY,
                &files.serialize().map_err(LoadError::Serialization)?,
            )
            .await
    }

    /// Delete a source file from flash.
    async fn delete(&mut self, name: &Name) -> Result<(), LoadError> {
        let mut files = self.storage.fetch_files().await?;

        files.remove(name);
        self.save_files(&files).await?;

        self.storage.delete(&key_from_name(name)).await
    }

    /// Rename a source file while preserving its contents.
    async fn rename(&mut self, old_name: Name, new_name: Name) -> Result<(), LoadError> {
        let mut files = self.storage.fetch_files().await?;

        let mut source = self.load(old_name.clone()).await?;
        source.name = new_name.clone();

        let mut buffer = [0; MAX_SIZE];

        self.storage.delete(&key_from_name(&old_name)).await?;
        self.storage
            .save(
                &key_from_name(&new_name),
                source
                    .serialize(&mut buffer)
                    .map_err(LoadError::Serialization)?,
            )
            .await?;

        files.remove(&old_name);
        files.create(new_name)?;
        self.save_files(&files).await?;

        Ok(())
    }

    /// Load a source file from flash.
    async fn load(&mut self, name: Name) -> Result<editor::source::Source, LoadError> {
        let serialized: [u8; MAX_SIZE] = self.storage.load(&key_from_name(&name)).await?;

        Source::deserialize(name, &serialized).map_err(LoadError::Serialization)
    }
}

impl embedded_gui::effect::Effect for Effect {
    type Msg = Msg;
    type Context = Context;

    /// Execute the effect and return the resulting application message, if any.
    async fn run(self, context: &mut Context) -> Option<Msg> {
        let msg = match self {
            Effect::Save(name, bytes) => Msg::SaveCompleted(context.save(name, bytes).await),
            Effect::Load(name) => Msg::LoadCompleted(context.load(name).await),
            Effect::Delete(name) => Msg::DeleteCompleted(context.delete(&name).await),
            Effect::Rename { old, new } => Msg::RenameCompleted(context.rename(old, new).await),

            Effect::FetchFiles => Msg::Page(PageMsg::SelectFile(select_file::Msg::FilesReceived(
                context
                    .storage
                    .load(&storage::FILE_LIST_KEY)
                    .await
                    .and_then(|serialized: [u8; MAX_SIZE]| {
                        Files::deserialize(&serialized).map_err(LoadError::Serialization)
                    }),
            ))),

            Effect::SetPanel(panel) => {
                PANEL.store(panel, Ordering::Relaxed);
                return None;
            }

            Effect::BuildWavetable(name, source_code) => {
                match calc::parser::parse(&context.ast_arena, &source_code) {
                    Ok(expr) => {
                        match Wavetable::try_from_fn(|x| {
                            let outer_scope = rpds::list![
                                rpds::ht_map!["x" => calc::interpreter::Value::Number(x)]
                            ];

                            let (dynamic_value, _) =
                                calc::interpreter::eval(&expr, outer_scope).unwrap();

                            match dynamic_value {
                                calc::interpreter::Value::Number(value) => Ok(value),
                                _ => Err(calc::interpreter::diagnostic::Error::new(
                                    calc::interpreter::diagnostic::ErrorKind::Expected(
                                        calc::interpreter::Type::Number,
                                        dynamic_value,
                                    ),
                                    (0, source_code.len()),
                                )),
                            }
                        }) {
                            Ok(wavetable) => {
                                usb::set_wavetable(wavetable);

                                Msg::WavetableBuilt(Ok(()))
                            }
                            Err(error) => {
                                let diagnostic: Diagnostic<()> = error.into();
                                let error_text =
                                    display_diagnostic(diagnostic, &name, &source_code);

                                Msg::WavetableBuilt(Err(error_text))
                            }
                        }
                    }
                    Err(error) => {
                        let diagnostic: Diagnostic<()> = error.into();
                        let error_text = display_diagnostic(diagnostic, &name, &source_code);

                        Msg::WavetableBuilt(Err(error_text))
                    }
                }
            }
        };
        context.ast_arena.reset();
        Some(msg)
    }
}

/// Render an interpreter diagnostic into a human-readable error message.
fn display_diagnostic(diagnostic: Diagnostic<()>, name: &Name, source_code: &str) -> String {
    codespan_reporting::term::emit_into_string(
        &codespan_reporting::term::Config::default(),
        &codespan_reporting::files::SimpleFile::new(
            fixed_str_to_str(name).unwrap_or("<corrupted>"),
            source_code,
        ),
        &diagnostic,
    )
    .unwrap_or("Error displaying diagnostic".to_string())
}
