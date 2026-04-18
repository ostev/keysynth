#![no_std]

use lalrpop_util::lalrpop_mod;
extern crate alloc;

// mod interpreter;
pub mod parser;
mod peek_next;

lalrpop_mod!(
    #[allow(clippy::ptr_arg)]
    #[allow(unused_parens)]
    #[rustfmt::skip]
    grammar
);
