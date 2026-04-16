use bumpalo::collections::Vec;

use crate::position::Position;

pub struct Expr<'a, 's> {
    pub kind: ExprKind<'a, 's>,
    pub position: Position<'s>,
}

pub enum ExprKind<'a, 's> {
    Identifier(&'s str),
    Literal(Literal),
    /// Represents an `if ... then ... else ...` expression.
    Branch(Branch<'a, 's>),
    Function(Function<'a, 's>),
    /// Represents a function application.
    Application(&'a Expr<'a, 's>, Vec<'a, &'a Expr<'a, 's>>),
    /// Represents a `let ... in ...` expression.
    Let(Let<'a, 's>),
    /// Represents a binary operator expression like `x + 3` or
    /// `x^n`.
    BinaryOp(&'a Expr<'a, 's>, BinaryOpKind, &'a Expr<'a, 's>),
    /// Represents a unary operator expression like `-x`.
    UnaryOp(UnaryOpKind, &'a Expr<'a, 's>),
}

pub struct Assignment<'a, 's> {
    pub identifier: &'s str,
    pub expr: &'a Expr<'a, 's>,
}

pub enum Literal {
    Number(f32),
    Boolean(bool),
}

pub struct Branch<'a, 's> {
    pub predicate: &'a Expr<'a, 's>,
    pub if_true: &'a Expr<'a, 's>,
    pub otherwise: &'a Expr<'a, 's>,
}

pub struct Function<'a, 's> {
    pub args: Vec<'a, &'s str>,
    pub body: &'a Expr<'a, 's>,
}

pub struct Let<'a, 's> {
    pub assignments: Vec<'a, Assignment<'a, 's>>,
    pub body: &'a Expr<'a, 's>,
}

pub enum BinaryOpKind {
    Numeric(NumericBinaryOp),
    Comparison(ComparisonOp),
}

pub enum NumericBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Exp,
}

pub enum ComparisonOp {
    Equals,
    NotEquals,
    LessThan,
    LessThanOrEquals,
    GreaterThan,
    GreaterThanOrEquals,
}

pub enum UnaryOpKind {
    Negate,
}
