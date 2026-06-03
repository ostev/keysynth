mod clipboard;
mod history;
mod position;
pub mod source;
mod state;
mod view;

pub use source::MAX_SIZE;
pub use state::EditorState as State;
pub use state::Msg;
pub use view::EditorView as View;
