use crate::peek_next::{IteratorExt, PeekableNext};
use core::str::CharIndices;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    UnexpectedCharacter { index: usize },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token<'s> {
    // Assignment separators
    Newline,
    Semicolon,
    // Literals
    Identifier(&'s str),
    Number(f32),
    True,
    False,
    // Functions
    Fn,
    LeftParen,
    RightParen,
    Comma,
    Arrow,
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
    BangEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    // Branching
    If,
    Then,
    Else,
    // let ... in ...
    Let,
    In,
}

pub type Spanned<'s> = Result<(usize, Token<'s>, usize), Error>;

pub struct Scanner<'s> {
    input: &'s str,
    chars: PeekableNext<CharIndices<'s>>,
}

impl<'s> Scanner<'s> {
    pub fn new(input: &'s str) -> Scanner<'s> {
        Scanner {
            chars: input.char_indices().peekable_next(),
            input,
        }
    }

    fn make_spanned(token: Token<'s>, index: usize, length: usize) -> Spanned<'s> {
        Ok((index, token, index + length))
    }

    fn make_identifier_or_keyword(&mut self, index: usize, length: usize) -> Spanned<'s> {
        let text = &self.input[index..=index + length];
        let token = match text {
            "if" => Token::If,
            "then" => Token::Then,
            "else" => Token::Else,
            "true" => Token::True,
            "false" => Token::False,
            "let" => Token::Let,
            "in" => Token::In,
            "fn" => Token::Fn,
            _ => Token::Identifier(text),
        };

        Ok((index, token, index + length))
    }

    fn is_insignificant_whitespace(&mut self) -> bool {
        self.chars
            .peek()
            .map(|(_index, char)| {
                let is_comment = *char == '/'
                    && self
                        .chars
                        .peek_next()
                        .map(|(_, char_2)| *char_2 == '/')
                        .unwrap_or(false);

                char.is_whitespace() || is_comment
            })
            .unwrap_or(false)
    }

    fn skip_whitespace(&mut self) {
        while self.is_insignificant_whitespace() {
            self.chars.next();
        }
    }

    fn next_matches(&mut self, expected: char) -> bool {
        let matches = self
            .chars
            .peek()
            .map(|(_, char)| *char == expected)
            .unwrap_or(false);

        if matches {
            self.chars.next();

            true
        } else {
            false
        }
    }

    fn next_matches_predicate(&mut self, predicate: impl Fn(&char) -> bool) -> bool {
        let matches = self
            .chars
            .peek()
            .map(|(_, char)| predicate(char))
            .unwrap_or(false);

        if matches {
            self.chars.next();

            true
        } else {
            false
        }
    }

    fn next_map(
        &mut self,
        f: impl Fn(char) -> Option<Spanned<'s>>,
        default: Spanned<'s>,
    ) -> Spanned<'s> {
        let peek = self.chars.peek();
        let value = peek.and_then(|(_, char)| f(*char));

        if let Some(_) = value {
            self.chars.next();
        }

        value.unwrap_or(default)
    }

    fn count_matching(&mut self, predicate: impl Fn(&char) -> bool) -> usize {
        let mut count = 0;

        while self.next_matches_predicate(&predicate) {
            count += 1;
        }

        count
    }
}

impl<'s> Iterator for Scanner<'s> {
    type Item = Spanned<'s>;

    fn next(&mut self) -> Option<Spanned<'s>> {
        self.skip_whitespace();
        let char = self.chars.next();

        char.map(|(index, char)| match char {
            '\n' => {
                let token = Self::make_spanned(Token::Newline, index, 1);

                token
            }
            ';' => Self::make_spanned(Token::Semicolon, index, 1),
            '(' => Self::make_spanned(Token::LeftParen, index, 1),
            ')' => Self::make_spanned(Token::RightParen, index, 1),
            ',' => Self::make_spanned(Token::Comma, index, 1),
            '-' => Self::make_spanned(Token::Minus, index, 1),
            '+' => Self::make_spanned(Token::Plus, index, 1),
            '/' => Self::make_spanned(Token::Slash, index, 1),
            '*' => Self::make_spanned(Token::Star, index, 1),
            '^' => Self::make_spanned(Token::Caret, index, 1),
            '=' => self.next_map(
                |peek| match peek {
                    '=' => Some(Self::make_spanned(Token::EqualEqual, index, 2)),
                    '>' => Some(Self::make_spanned(Token::Arrow, index, 2)),
                    _ => None,
                },
                Self::make_spanned(Token::Equal, index, 1),
            ),
            '!' => {
                if self.next_matches('=') {
                    Self::make_spanned(Token::BangEqual, index, 2)
                } else {
                    Err(Error::UnexpectedCharacter { index })
                }
            }
            '>' => {
                if self.next_matches('=') {
                    Self::make_spanned(Token::GreaterEqual, index, 2)
                } else {
                    Self::make_spanned(Token::Greater, index, 1)
                }
            }
            '<' => {
                if self.next_matches('=') {
                    Self::make_spanned(Token::LessEqual, index, 2)
                } else {
                    Self::make_spanned(Token::Less, index, 1)
                }
            }
            _ => {
                if char.is_ascii_digit() {
                    let count = self.count_matching(char::is_ascii_digit);
                    let text = &self.input[index..=index + count];
                    Self::make_spanned(Token::Number(text.parse().unwrap()), index, count)
                } else if char.is_ascii_alphabetic() || char == '_' {
                    let count =
                        self.count_matching(|char| char.is_ascii_alphanumeric() || *char == '_');
                    self.make_identifier_or_keyword(index, count)
                } else {
                    Err(Error::UnexpectedCharacter { index })
                }
            }
        })
    }
}
