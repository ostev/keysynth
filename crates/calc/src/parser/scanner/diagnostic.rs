use codespan_reporting::diagnostic::{Diagnostic, Label};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    UnexpectedCharacter { index: usize },
}

impl Into<Diagnostic<()>> for Error {
    fn into(self) -> Diagnostic<()> {
        match self {
            Error::UnexpectedCharacter { index } => {
                let label = Label::primary((), index..index)
                    .with_message("I don't understand this character!");
                Diagnostic::error().with_code("E001").with_label(label)
            }
        }
    }
}
