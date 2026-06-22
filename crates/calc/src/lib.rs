#![no_std]

use lalrpop_util::{ParseError, lalrpop_mod};

use crate::{
    interpreter::Value,
    parser::scanner::{self, Token},
};
extern crate alloc;

pub mod interpreter;
pub mod parser;
mod peek_next;

lalrpop_mod!(
    #[allow(clippy::ptr_arg)]
    #[allow(unused_parens)]
    #[rustfmt::skip]
    grammar
);

/// Represents an error that can occur while executing the program.
#[derive(Debug)]
pub enum ExecutionError<'a, 's> {
    ParseError(ParseError<usize, Token<'s>, scanner::diagnostic::Error>),
    RuntimeError(interpreter::diagnostic::Error<'a, 's>),
}
