use bumpalo::Bump;
use lalrpop_util::ParseError;

use crate::{
    grammar,
    parser::{
        ast::{Expr, Spanned},
        scanner::{Scanner, Token},
    },
};

extern crate alloc;

pub mod ast;
pub mod scanner;

pub fn parse<'a: 's, 's>(
    bump: &'a Bump,
    input: &'s str,
) -> Result<Spanned<'a, 's>, ParseError<usize, Token<'s>, scanner::Error>> {
    grammar::ExprParser::new().parse(input, bump, Scanner::new(input))
}
