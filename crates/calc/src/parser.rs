use bumpalo::Bump;

use crate::{
    grammar,
    parser::{
        ast::Spanned,
        scanner::Scanner,
    },
};

use diagnostic::Error;

extern crate alloc;

pub mod ast;
pub mod diagnostic;
pub mod scanner;

pub fn parse<'a, 's>(bump: &'a Bump, input: &'s str) -> Result<Spanned<'a, 's>, Error<'s>> {
    grammar::ExprParser::new()
        .parse(input, bump, Scanner::new(input))
        .map_err(Error)
}
