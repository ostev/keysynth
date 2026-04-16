pub mod ast;
mod scanner;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use bumpalo::Bump;
use chumsky::{
    input::{Input as _, MappedInput},
    pratt::*,
    prelude::*,
};

use crate::parser::{
    ast::{Assignment, BinaryOpKind, Expr, Let, Literal, NumericBinaryOp},
    scanner::Token,
};

pub fn parser<'a, 'tokens, 's: 'tokens + 'a>(
    arena: &'a Bump,
) -> impl Parser<
    's,
    Vec<Spanned<Token<'s>>>,
    &'s str,
    Expr<'a, 's>,
    // extra::Err<Rich<'tokens, Token<'s>>>,
> {
    recursive(|expr| {
        // let ident = select_ref! {Token::Identifier(identifier) => *identifier};

        // let assignment = ident
        //     .then_ignore(just(Token::Equal))
        //     .then(expr.clone())
        //     .map(|(identifier, value)| Assignment {
        //         identifier,
        //         expr: value,
        //     });

        // let let_expr = just(Token::Let)
        //     .ignore_then(
        //         assignment
        //             .separated_by(just(Token::Newline))
        //             .allow_leading()
        //             .allow_trailing()
        //             .collect::<Vec<_>>(),
        //     )
        //     .then_ignore(just(Token::In))
        //     .then(expr.clone())
        //     .map(|(assignments, body)| Let { assignments, body });

        // let atom = choice((
        //     // 1.23
        //     select_ref! {Token::Number(number) => *number}
        //         .map(|number| arena.alloc(Expr::Literal(Literal::Number(number)))),
        //     // true
        //     just(Token::True).map(|_| arena.alloc(Expr::Literal(Literal::Boolean(true)))),
        //     // false
        //     just(Token::False).map(|_| arena.alloc(Expr::Literal(Literal::Boolean(false)))),
        //     // let
        //     //   y = 5
        //     //   z = 5*y
        //     // in
        //     //   y + z + 3
        //     // let_expr.map(|let_expr| arena.alloc(Expr::Let(let_expr))),
        // ));

        // choice((
        //     atom.spanned(),
        //     // expr.nested_in(
        //     //     select_ref! {Token::Parens(inside) = e => inside.split_spanned(e.span())},
        //     // ),
        // ))
        // // .pratt([infix(left(10), just(Token::Star), |x, _, y, _| {
        // //     arena.alloc(Expr::BinaryOp(
        // //         x,
        // //         BinaryOpKind::Numeric(NumericBinaryOp::Mul),
        // //         y,
        // //     ))
        // // })])
        // .labelled("expression")
        // .as_context()
        just(Token::True).to(Expr::<'a, 's>::Literal(Literal::Number(3.0)))
        // .spanned()
    })
}
