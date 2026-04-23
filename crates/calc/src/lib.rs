#![no_std]

use core::f32;

use bumpalo::Bump;
use lalrpop_util::{ParseError, lalrpop_mod};
use micromath::F32Ext;
use rpds::{HashTrieMap, List, ht_map};

use crate::{
    interpreter::{Error, ErrorKind, Value},
    parser::{
        ast::Span,
        scanner::{self, Token},
    },
};
extern crate alloc;

mod diagonostics;
pub mod interpreter;
pub mod parser;
mod peek_next;

lalrpop_mod!(
    #[allow(clippy::ptr_arg)]
    #[allow(unused_parens)]
    #[rustfmt::skip]
    grammar
);

#[derive(Debug)]
pub enum ExecutionError<'v, 's> {
    ParseError(ParseError<usize, Token<'s>, scanner::Error>),
    RuntimeError(interpreter::diagnostics::Error<'v, 's>),
}

pub fn run<'a, 's: 'a>(
    ast_arena: &'a Bump,
    input: &'s str,
) -> Result<Value<'a, 's>, ExecutionError<'a, 's>> {
    let parsed =
        ast_arena.alloc(parser::parse(ast_arena, input).map_err(ExecutionError::ParseError)?);
    let (evaluated, _) =
        interpreter::eval(parsed, List::new()).map_err(ExecutionError::RuntimeError)?;

    Ok(evaluated)
}
