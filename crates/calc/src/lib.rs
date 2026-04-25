#![no_std]

use core::f32;

use alloc::rc::Rc;
use bumpalo::{Bump, boxed::Box};
use lalrpop_util::{ParseError, lalrpop_mod};
use micromath::F32Ext;
use rpds::{HashTrieMap, List, ht_map};

use crate::{
    interpreter::{OwnedValue, Value},
    parser::{
        ast::Span,
        scanner::{self, Token},
    },
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

#[derive(Debug)]
pub enum ExecutionError<'a, 's> {
    ParseError(ParseError<usize, Token<'s>, scanner::diagnostic::Error>),
    RuntimeError(interpreter::diagnostic::Error<'a, 's>),
}

// pub fn run<'s>(input: &'s str) -> Result<Option<OwnedValue>, ExecutionError<'_, 's>> {
//     // let concrete = evaluated.as_concrete();

//     // Ok(concrete)
//     Ok(None)
// }
