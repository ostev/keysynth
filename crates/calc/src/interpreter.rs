use core::ops;

use bumpalo::{
    Bump,
    collections::{CollectIn, Vec},
};
use micromath::F32Ext;
use rpds::HashTrieMap;

use crate::{
    parser::ast::{BinaryOpKind, Comparison, Expr, ExprKind, Let, Literal, UnaryOpKind},
    position::Position,
};

#[derive(Clone, Copy)]
pub enum Value<'v, 's> {
    Number(f32),
    Boolean(bool),
    Function(FunctionValue<'v, 's>),
}

impl<'v, 's> Value<'v, 's> {
    pub fn map_number(
        self,
        position: Position<'s>,
        f: impl Fn(f32) -> f32,
    ) -> Result<Self, Error<'v, 's>> {
        match self {
            Value::Number(number) => Ok(Value::Number(f(number))),
            _ => Err(Error::new(
                ErrorKind::Expected(Type::Number, self),
                position,
            )),
        }
    }
    pub fn flatmap_number(
        self,
        position: Position<'s>,
        f: impl Fn(f32) -> EvalResult<'v, 's>,
    ) -> EvalResult<'v, 's> {
        match self {
            Value::Number(number) => f(number),
            _ => Err(Error::new(
                ErrorKind::Expected(Type::Number, self),
                position,
            )),
        }
    }

    pub fn flatmap_boolean(
        self,
        position: Position<'s>,
        f: impl Fn(bool) -> EvalResult<'v, 's>,
    ) -> EvalResult<'v, 's> {
        match self {
            Value::Boolean(boolean) => f(boolean),
            _ => Err(Error::new(
                ErrorKind::Expected(Type::Boolean, self),
                position,
            )),
        }
    }

    pub fn map_boolean(
        self,
        position: Position<'s>,
        f: impl Fn(bool) -> bool,
    ) -> Result<Self, Error<'v, 's>> {
        match self {
            Value::Boolean(boolean) => Ok(Value::Boolean(f(boolean))),
            _ => Err(Error::new(
                ErrorKind::Expected(Type::Boolean, self),
                position,
            )),
        }
    }
}

pub enum Type {
    Boolean,
    Number,
    Function,
}

pub type FunctionValue<'v, 's> =
    &'v (dyn Fn(Position<'s>, Vec<'v, Value<'v, 's>>) -> EvalResult<'v, 's> + 'v);

/// A scope is defined as a stack of mappings between a name and a potentially evaluated value.
/// When the code contains an identifier, the interpreter checks each scope in the stack
/// (starting at the front) to see if it contains the identifier. If the identifier is a thunk
/// (i.e. has not yet been evaluated because of assignment ordering), the assignment that
/// references it will become a thunk.
pub type Scopes<'v, 's> = rpds::List<rpds::HashTrieMap<&'s str, Value<'v, 's>>>;

pub type EvalResult<'v, 's> = Result<(Value<'v, 's>, Scopes<'v, 's>), Error<'v, 's>>;

pub struct Error<'v, 's> {
    pub kind: ErrorKind<'v, 's>,
    pub position: Position<'s>,
}

impl<'v, 's> Error<'v, 's> {
    pub const fn new(kind: ErrorKind<'v, 's>, position: Position<'s>) -> Self {
        Self { kind, position }
    }
}

pub enum ErrorKind<'v, 's> {
    IdentifierNotInScope,
    Expected(Type, Value<'v, 's>),
    IncorrectArity { expected: usize, received: usize },
}

pub fn eval<'a: 'v, 'v, 's>(
    bump: &'v Bump,
    expr: &'a Expr<'a, 's>,
    scopes: Scopes<'v, 's>,
) -> EvalResult<'v, 's> {
    Ok(match &expr.kind {
        ExprKind::Identifier(identifier) => {
            let value = scopes
                .iter()
                .find_map(|scope| scope.get(identifier).copied())
                .ok_or(Error::new(ErrorKind::IdentifierNotInScope, expr.position))?;

            (value, scopes)
        }
        ExprKind::Literal(literal) => (
            match literal {
                Literal::Number(number) => Value::Number(*number),
                Literal::Boolean(boolean) => Value::Boolean(*boolean),
            },
            scopes,
        ),
        ExprKind::Comparison(Comparison {
            predicate,
            if_true,
            otherwise,
        }) => {
            let (condition, _) = eval(bump, *predicate, scopes.clone())?;

            let (value, _) = condition.flatmap_boolean(expr.position, |boolean| {
                if boolean {
                    eval(bump, if_true, scopes.clone())
                } else {
                    eval(bump, otherwise, scopes.clone())
                }
            })?;

            (value, scopes)
        }
        ExprKind::Function(function) => {
            let cloned_scopes = scopes.clone();
            let f = move |callsite: Position<'s>, args: Vec<'v, Value<'v, 's>>| {
                if args.len() != function.args.len() {
                    Err(Error::new(
                        ErrorKind::IncorrectArity {
                            expected: function.args.len(),
                            received: args.len(),
                        },
                        callsite,
                    ))
                } else {
                    let body_scope = scopes.clone().push_front(
                        function
                            .args
                            .iter()
                            .map(|name| *name)
                            .zip(args.into_iter())
                            .collect(),
                    );

                    eval(bump, function.body, body_scope)
                }
            };

            (Value::Function(bump.alloc(f)), cloned_scopes)
        }
        ExprKind::Application(function_expr, args) => {
            let evaluated_args: Vec<'v, Value<'v, 's>> = args
                .iter()
                .map(|arg| eval(bump, arg, scopes.clone()).map(|(value, _)| value))
                .collect_in::<Result<_, Error>>(bump)?;

            let (function_value, _) = eval(bump, *function_expr, scopes.clone())?;

            match function_value {
                Value::Function(function) => {
                    let (value, _) = function(expr.position, evaluated_args)?;
                    (value, scopes)
                }
                _ => Err(Error::new(
                    ErrorKind::Expected(Type::Function, function_value),
                    expr.position,
                ))?,
            }
        }
        ExprKind::Let(Let { assignments, body }) => {
            let new_scope = assignments.iter().try_fold(
                HashTrieMap::new(),
                |mut accumulated_scope, assignment| {
                    let (value, _) = eval(bump, assignment.expr, scopes.clone())?;

                    accumulated_scope.insert_mut(assignment.identifier, value);
                    Ok(accumulated_scope)
                },
            )?;

            let (value, _) = eval(bump, expr, scopes.push_front(new_scope))?;

            (value, scopes)
        }
        ExprKind::BinaryOp(left_expr, op, right_expr) => {
            let operator: fn(f32, f32) -> f32 = match op {
                BinaryOpKind::Add => ops::Add::add,
                BinaryOpKind::Sub => ops::Sub::sub,
                BinaryOpKind::Mul => ops::Mul::mul,
                BinaryOpKind::Div => ops::Div::div,
                BinaryOpKind::Exp => |value, exponent| {
                    if exponent.trunc() == exponent {
                        value.powi(exponent as i32)
                    } else {
                        value.powf(exponent)
                    }
                },
            };

            let (left, _) = eval(bump, expr, scopes.clone())?;

            let output = match left {
                Value::Number(x) => {
                    let (right, _) = eval(bump, expr, scopes.clone())?;

                    match right {
                        Value::Number(y) => Ok(Value::Number(operator(x, y))),

                        _ => Err(Error::new(
                            ErrorKind::Expected(Type::Number, right),
                            right_expr.position,
                        )),
                    }
                }
                _ => Err(Error::new(
                    ErrorKind::Expected(Type::Number, left),
                    left_expr.position,
                )),
            }?;

            (output, scopes)
        }
        ExprKind::UnaryOp(op, expr) => match op {
            UnaryOpKind::Negate => {
                let (value, _) = eval(bump, expr, scopes.clone())?;
                let negated = value.map_number(expr.position, ops::Neg::neg)?;
                (negated, scopes)
            }
        },
    })
}
