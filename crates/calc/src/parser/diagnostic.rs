use alloc::{format, string::String, vec::Vec};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use itertools::Itertools;
use lalrpop_util::ParseError;

use crate::parser::scanner::{self, Token};

pub struct Error<'s>(pub ParseError<usize, Token<'s>, scanner::diagnostic::Error>);

fn label(location: usize) -> Label<()> {
    Label::primary((), location..location)
}

fn join_tokens(tokens: &Vec<String>) -> String {
    tokens
        .split_last()
        .map(|(last, rest)| match rest.len() {
            1 => last.clone(),
            _ => format!("{} or {}", rest.join(", "), last),
        })
        .unwrap_or(String::new())
}

impl<'s> Into<Diagnostic<()>> for Error<'s> {
    fn into(self) -> Diagnostic<()> {
        let Error(error) = self;

        match error {
            ParseError::InvalidToken { location } => {
                Diagnostic::error().with_code("E201").with_label(
                    label(location).with_message("I didn't expect to see this token here!"),
                )
            }
            ParseError::UnrecognizedEof { location, expected } => {
                let expected_text = join_tokens(&expected);
                Diagnostic::error()
                    .with_code("E202")
                    .with_label(label(location).with_message(format!(
                        "Your code ended when I expected {expected_text} instead!"
                    )))
            }
            ParseError::UnrecognizedToken {
                token: (start, token, end),
                expected,
            } => {
                let expected_text = join_tokens(&expected);
                Diagnostic::error().with_code("E203").with_label(
                    Label::primary((), start..(end + 1)).with_message(format!(
                        "Your code contained a {token} when I expected {expected_text} instead!"
                    )),
                )
            }
            ParseError::ExtraToken {
                token: (start, token, end),
            } => Diagnostic::error().with_code("E204").with_label(
                Label::primary((), start..(end + 1))
                    .with_message(format!("I didn't expect to see {token} here!")),
            ),
            ParseError::User { error } => error.into(),
        }
    }
}
