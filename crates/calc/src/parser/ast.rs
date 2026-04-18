use alloc::vec::Vec;
use bumpalo::boxed::Box;

#[derive(Debug)]
pub enum Expr<'a, 's> {
    Identifier(&'s str),
    Literal(Literal),
    /// Represents an `if ... then ... else ...` expression.
    Branch(Branch<'a, 's>),
    Function(Function<'a, 's>),
    /// Represents a function application.
    Application(Box<'a, Expr<'a, 's>>, Vec<Expr<'a, 's>>),
    /// Represents a `let ... in ...` expression.
    Let(Let<'a, 's>),
    /// Represents a binary operator expression like `x + 3` or
    /// `x^n`.
    BinaryOp(Box<'a, Expr<'a, 's>>, BinaryOp, Box<'a, Expr<'a, 's>>),
    /// Represents a unary operator expression like `-x`.
    UnaryOp(UnaryOpKind, Box<'a, Expr<'a, 's>>),
}

#[derive(Debug)]
pub struct Assignment<'a, 's> {
    pub identifier: &'s str,
    pub expr: Expr<'a, 's>,
}

#[derive(Clone, Copy, Debug)]
pub enum Literal {
    Number(f32),
    Boolean(bool),
}

#[derive(Debug)]
pub struct Branch<'a, 's> {
    pub predicate: Box<'a, Expr<'a, 's>>,
    pub if_true: Box<'a, Expr<'a, 's>>,
    pub otherwise: Box<'a, Expr<'a, 's>>,
}

#[derive(Debug)]
pub struct Function<'a, 's> {
    pub args: Vec<&'s str>,
    pub body: Box<'a, Expr<'a, 's>>,
}

#[derive(Debug)]
pub struct Let<'a, 's> {
    pub assignments: Vec<Assignment<'a, 's>>,
    pub body: Box<'a, Expr<'a, 's>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinaryOp {
    Numeric(NumericBinaryOp),
    Comparison(ComparisonBinaryOp),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NumericBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Exp,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ComparisonBinaryOp {
    Equals,
    NotEquals,
    LessThan,
    LessThanOrEquals,
    GreaterThan,
    GreaterThanOrEquals,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnaryOpKind {
    Negate,
}
