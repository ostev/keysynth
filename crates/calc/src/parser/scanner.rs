use core::{
    fmt::{self, Display},
    str::CharIndices,
};

use crate::{
    peek_next::{IteratorExt, PeekableNext},
    position::Position,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    UnexpectedCharacter,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    // Whitespace
    Newline,
    // Literals
    Identifier,
    Number,
    True,
    False,
    // Functions
    LeftParen,
    RightParen,
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
    // Error
    Error(Error),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token<'s> {
    pub kind: TokenKind,
    pub position: Position<'s>,
}

impl<'s> Display for Token<'s> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TokenKind::Newline => write!(f, "Newline"),
            TokenKind::Identifier => write!(f, "Identifier({})", self.position.text),
            TokenKind::Number => write!(f, "Number({})", self.position.text),
            TokenKind::True => write!(f, "True"),
            TokenKind::False => write!(f, "False"),
            TokenKind::LeftParen => write!(f, "LeftParen"),
            TokenKind::RightParen => write!(f, "RightParen"),
            TokenKind::Comma => write!(f, "Comma"),
            TokenKind::Minus => write!(f, "Minus"),
            TokenKind::Plus => write!(f, "Plus"),
            TokenKind::Slash => write!(f, "Slash"),
            TokenKind::Star => write!(f, "Star"),
            TokenKind::Caret => write!(f, "Caret"),
            TokenKind::Equal => write!(f, "Equal"),
            TokenKind::EqualEqual => write!(f, "EqualEqual"),
            TokenKind::Greater => write!(f, "Greater"),
            TokenKind::GreaterEqual => write!(f, "GreaterEqual"),
            TokenKind::Less => write!(f, "Less"),
            TokenKind::LessEqual => write!(f, "LessEqual"),
            TokenKind::If => write!(f, "If"),
            TokenKind::Then => write!(f, "Then"),
            TokenKind::Else => write!(f, "Else"),
            TokenKind::Error(error) => write!(f, "Error({:?})", error),
        }
    }
}

pub struct Scanner<'s> {
    input: &'s str,
    chars: PeekableNext<CharIndices<'s>>,

    line: usize,
    column: usize,
}

impl<'s> Scanner<'s> {
    pub fn new(input: &'s str) -> Scanner<'s> {
        Scanner {
            chars: input.char_indices().peekable_next(),
            line: 0,
            column: 0,
            input,
        }
    }

    fn make_token(&mut self, kind: TokenKind, index: usize, length: usize) -> Token<'s> {
        self.column += length;

        Token {
            kind,
            position: Position {
                index,
                line: self.line,
                column: self.column,
                text: &self.input[index..index + length],
            },
        }
    }

    fn make_identifier_or_keyword(&mut self, index: usize, length: usize) -> Token<'s> {
        self.column += length;

        let text = &self.input[index..index + length];
        let kind = match text {
            "if" => TokenKind::If,
            "then" => TokenKind::Then,
            "else" => TokenKind::Else,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            _ => TokenKind::Identifier,
        };

        Token {
            kind,
            position: Position {
                index,
                line: self.line,
                column: self.column,
                text,
            },
        }
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

                (*char != '\n' && char.is_whitespace()) || is_comment
            })
            .unwrap_or(false)
    }

    fn skip_whitespace(&mut self) {
        while self.is_insignificant_whitespace() {
            self.chars.next().map(|_| {
                self.column += 1;
            });
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

    fn count_matching(&mut self, predicate: impl Fn(&char) -> bool) -> usize {
        let mut count = 0;

        while self.next_matches_predicate(&predicate) {
            count += 1;
        }

        count
    }
}

impl<'s> Iterator for Scanner<'s> {
    type Item = Token<'s>;

    fn next(&mut self) -> Option<Token<'s>> {
        self.skip_whitespace();
        let char = self.chars.next();

        char.map(|(index, char)| match char {
            '\n' => {
                let token = self.make_token(TokenKind::Newline, index, 1);

                self.line += 1;
                self.column = 0;
                token
            }
            '(' => self.make_token(TokenKind::LeftParen, index, 1),
            ')' => self.make_token(TokenKind::RightParen, index, 1),
            ',' => self.make_token(TokenKind::Comma, index, 1),
            '-' => self.make_token(TokenKind::Minus, index, 1),
            '+' => self.make_token(TokenKind::Plus, index, 1),
            '/' => self.make_token(TokenKind::Slash, index, 1),
            '*' => self.make_token(TokenKind::Star, index, 1),
            '^' => self.make_token(TokenKind::Caret, index, 1),
            '=' => {
                if self.next_matches('=') {
                    self.make_token(TokenKind::EqualEqual, index, 2)
                } else {
                    self.make_token(TokenKind::Equal, index, 1)
                }
            }
            '>' => {
                if self.next_matches('=') {
                    self.make_token(TokenKind::GreaterEqual, index, 2)
                } else {
                    self.make_token(TokenKind::Greater, index, 1)
                }
            }
            '<' => {
                if self.next_matches('=') {
                    self.make_token(TokenKind::LessEqual, index, 2)
                } else {
                    self.make_token(TokenKind::Less, index, 1)
                }
            }
            _ => {
                if char.is_ascii_digit() {
                    let count = self.count_matching(char::is_ascii_digit);
                    self.make_token(TokenKind::Number, index, count)
                } else if char.is_ascii_alphabetic() || char == '_' {
                    let count =
                        self.count_matching(|char| char.is_ascii_alphanumeric() || *char == '_');
                    self.make_token(TokenKind::Identifier, index, count)
                } else {
                    self.make_token(TokenKind::Error(Error::UnexpectedCharacter), index, 1)
                }
            }
        })
    }
}
