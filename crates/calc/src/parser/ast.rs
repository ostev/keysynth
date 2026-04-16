use alloc::vec::Vec;

use crate::position::Position;

#[derive(Clone, Debug)]
pub enum Expr<'a, 's> {
    Identifier(&'s str),
    Literal(Literal),
    /// Represents an `if ... then ... else ...` expression.
    Branch(Branch<'a, 's>),
    Function(Function<'a, 's>),
    /// Represents a function application.
    Application(&'a Expr<'a, 's>, Vec<&'a Expr<'a, 's>>),
    /// Represents a `let ... in ...` expression.
    Let(Let<'a, 's>),
    /// Represents a binary operator expression like `x + 3` or
    /// `x^n`.
    BinaryOp(&'a Expr<'a, 's>, BinaryOpKind, &'a Expr<'a, 's>),
    /// Represents a unary operator expression like `-x`.
    UnaryOp(UnaryOpKind, &'a Expr<'a, 's>),
}

#[derive(Clone, Debug)]
pub struct Assignment<'a, 's> {
    pub identifier: &'s str,
    pub expr: &'a Expr<'a, 's>,
}

#[derive(Clone, Debug)]
pub enum Literal {
    Number(f32),
    Boolean(bool),
}

#[derive(Clone, Debug)]
pub struct Branch<'a, 's> {
    pub predicate: &'a Expr<'a, 's>,
    pub if_true: &'a Expr<'a, 's>,
    pub otherwise: &'a Expr<'a, 's>,
}

#[derive(Clone, Debug)]
pub struct Function<'a, 's> {
    pub args: Vec<&'s str>,
    pub body: &'a Expr<'a, 's>,
}

#[derive(Clone, Debug)]
pub struct Let<'a, 's> {
    pub assignments: Vec<Assignment<'a, 's>>,
    pub body: &'a Expr<'a, 's>,
}

#[derive(Clone, Debug)]
pub enum BinaryOpKind {
    Numeric(NumericBinaryOp),
    Comparison(ComparisonOp),
}

#[derive(Clone, Debug)]
pub enum NumericBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Exp,
}

#[derive(Clone, Debug)]
pub enum ComparisonOp {
    Equals,
    NotEquals,
    LessThan,
    LessThanOrEquals,
    GreaterThan,
    GreaterThanOrEquals,
}

#[derive(Clone, Debug)]
pub enum UnaryOpKind {
    Negate,
}
