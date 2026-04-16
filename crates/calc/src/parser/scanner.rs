use core::{
    fmt::{self, Display, write},
    str::CharIndices,
};

use alloc::vec::Vec;
use chumsky::{
    container::Container,
    input::{Input as _, MappedInput},
    pratt::*,
    prelude::*,
};

use crate::{
    peek_next::{IteratorExt, PeekableNext},
    position::Position,
};

#[derive(Clone, Debug, PartialEq)]
pub enum Token<'a> {
    // Whitespace
    Newline,
    // Literals
    Identifier(&'a str),
    Number(f32),
    True,
    False,
    // Functions
    Parens(Vec<Spanned<Self>>),
    Comma,
    // Arithmetic
    Minus,
    Plus,
    Slash,
    Star,
    Caret,
    // Assignment
    Equal,
    // Comparison
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    // Keywords
    If,
    Then,
    Else,
    // Let
    Let,
    In,
}

impl<'s> Display for Token<'s> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Newline => write!(f, "\n"),
            Token::Identifier(identifier) => write!(f, "Identifier({identifier})"),
            Token::Number(number) => write!(f, "Number({number})"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Comma => write!(f, ","),
            Token::Minus => write!(f, "-"),
            Token::Plus => write!(f, "+"),
            Token::Slash => write!(f, "/"),
            Token::Star => write!(f, "*"),
            Token::Caret => write!(f, "^"),
            Token::Equal => write!(f, "="),
            Token::EqualEqual => write!(f, "=="),
            Token::Greater => write!(f, ">"),
            Token::GreaterEqual => write!(f, ">="),
            Token::Less => write!(f, "<"),
            Token::LessEqual => write!(f, "<="),
            Token::If => write!(f, "if"),
            Token::Then => write!(f, "then"),
            Token::Else => write!(f, "else"),
            Token::Parens(interior) => write!(f, "(..)"),
            Token::Let => write!(f, "let"),
            Token::In => write!(f, "in"),
        }
    }
}

fn lexer<'a, 's>() -> impl Parser<'s, &'s str, Vec<Spanned<Token<'s>>>, extra::Err<Rich<'s, char>>>
{
    recursive(|token| {
        choice((
            // Newlines are significant
            text::newline().to(Token::Newline),
            // Keywords and identifiers
            text::ident().map(|text| match text {
                "let" => Token::Let,
                "in" => Token::In,
                "if" => Token::If,
                "then" => Token::Then,
                "else" => Token::Else,
                "true" => Token::True,
                "false" => Token::False,
                identifier => Token::Identifier(identifier),
            }),
            // Operators
            just("==").to(Token::EqualEqual),
            just("=").to(Token::Equal),
            just("+").to(Token::Plus),
            just("*").to(Token::Star),
            just("/").to(Token::Slash),
            just("^").to(Token::Caret),
            // Numbers
            text::int(10)
                .then(just('.').then(text::digits(10)).or_not())
                .to_slice()
                .map(|text: &str| Token::Number(text.parse().unwrap())),
            token
                .repeated()
                .collect()
                .delimited_by(just('('), just(')'))
                .labelled("token tree")
                .as_context()
                .map(Token::Parens),
        ))
        .spanned()
        .padded()
    })
    .repeated()
    .collect()
}
