use core::{cmp, f32, fmt, ops};

use alloc::rc::Rc;
use bumpalo::{
    Bump,
    boxed::Box,
    collections::{CollectIn, Vec},
};
use micromath::F32Ext;
use rpds::{HashTrieMap, List};

use crate::{
    interpreter::builtins::{BuiltinFunction, BuiltinValue, get_builtin},
    parser::ast::{
        BinaryOp, Branch, ComparisonBinaryOp, Expr, Let, Literal, NumericBinaryOp, Span, Spanned,
        UnaryOpKind,
    },
};

pub mod builtins;

#[derive(Clone)]
pub enum Value<'v, 's> {
    Number(f32),
    Boolean(bool),
    Function(FunctionValue<'v, 's>),
    BuiltinFunction(BuiltinFunction),
}

impl<'v, 's> fmt::Debug for Value<'v, 's> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(number) => write!(f, "{}", number),
            Self::Boolean(boolean) => write!(f, "{}", boolean),
            Self::Function(_) => write!(f, "{}", "<fn>"),
            Self::BuiltinFunction(_) => write!(f, "{}", "<builtin>"),
        }
    }
}

impl<'v, 's> From<BuiltinValue> for Value<'v, 's> {
    fn from(value: BuiltinValue) -> Self {
        match value {
            BuiltinValue::Number(num) => Value::Number(num),
            BuiltinValue::Function(f) => Value::BuiltinFunction(f),
        }
    }
}

impl<'v, 's> Value<'v, 's> {
    pub fn map_number(self, span: Span, f: impl Fn(f32) -> f32) -> Result<Self, Error<'v, 's>> {
        match self {
            Value::Number(number) => Ok(Value::Number(f(number))),
            _ => Err(Error::new(ErrorKind::Expected(Type::Number, self), span)),
        }
    }
    pub fn flatmap_number(
        self,
        span: Span,
        f: impl Fn(f32) -> EvalResult<'v, 's>,
    ) -> EvalResult<'v, 's> {
        match self {
            Value::Number(number) => f(number),
            _ => Err(Error::new(ErrorKind::Expected(Type::Number, self), span)),
        }
    }

    pub fn flatmap_boolean(
        self,
        span: Span,
        f: impl Fn(bool) -> EvalResult<'v, 's>,
    ) -> EvalResult<'v, 's> {
        match self {
            Value::Boolean(boolean) => f(boolean),
            _ => Err(Error::new(ErrorKind::Expected(Type::Boolean, self), span)),
        }
    }

    pub fn map_boolean(self, span: Span, f: &impl Fn(bool) -> bool) -> Result<Self, Error<'v, 's>> {
        match self {
            Value::Boolean(boolean) => Ok(Value::Boolean(f(boolean))),
            _ => Err(Error::new(ErrorKind::Expected(Type::Boolean, self), span)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Type {
    Boolean,
    Number,
    Function,
}

pub type FunctionValue<'v, 's> = Rc<dyn Fn(Span, Vec<'v, Value<'v, 's>>) -> EvalResult<'v, 's>>;

/// A scope is defined as a stack of mappings between a name and a potentially evaluated value.
/// When the code contains an identifier, the interpreter checks each scope in the stack
/// (starting at the front) to see if it contains the identifier. If the identifier is a thunk
/// (i.e. has not yet been evaluated because of assignment ordering), the assignment that
/// references it will become a thunk.
pub type Scopes<'v, 's> = rpds::List<rpds::HashTrieMap<&'s str, Value<'v, 's>>>;

pub type EvalResult<'v, 's> = Result<(Value<'v, 's>, Scopes<'v, 's>), Error<'v, 's>>;

#[derive(Debug)]
pub struct Error<'v, 's> {
    pub kind: ErrorKind<'v, 's>,
    pub span: (usize, usize),
}

impl<'v, 's> Error<'v, 's> {
    pub const fn new(kind: ErrorKind<'v, 's>, span: (usize, usize)) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug)]
pub enum ErrorKind<'v, 's> {
    IdentifierNotInScope,
    Expected(Type, Value<'v, 's>),
    CannotCompareFunctions,
    ComparisonBinaryOperatorTypeMismatch {
        left: Value<'v, 's>,
        right: Value<'v, 's>,
    },
    IncorrectArity {
        expected: usize,
        received: usize,
    },
    DivisionByZero,
    DivisionByInfinity,
    FractionalExponentWithNegativeBase {
        base: f32,
        exponent: f32,
    },
    OutsideFunctionDomain {
        domain: (f32, f32),
        value: f32,
    },
    EvenRootOfNegativeValue {
        root: f32,
        value: f32,
    },
    ArbitraryRootThatIsNotOddOfNegativeValue {
        root: f32,
        value: f32,
    },
    NegativeRoot {
        root: f32,
        value: f32,
    },
}

pub fn eval<'a: 'v, 'v, 's>(
    bump: &'v Bump,
    spanned: &'a Spanned<'a, 's>,
    scopes: Scopes<'v, 's>,
) -> EvalResult<'v, 's> {
    Ok(match &spanned.expr {
        Expr::Identifier(identifier) => {
            let value = match scopes
                .iter()
                .find_map(|scope| scope.get(identifier).cloned())
            {
                Some(defined_value) => Some(defined_value),
                None => get_builtin(*identifier).map(Into::into),
            }
            .ok_or(Error::new(ErrorKind::IdentifierNotInScope, spanned.span()))?;

            (value, scopes)
        }
        Expr::Literal(literal) => (
            match literal {
                Literal::Number(number) => Value::Number(*number),
                Literal::Boolean(boolean) => Value::Boolean(*boolean),
            },
            scopes,
        ),
        Expr::Branch(Branch {
            predicate,
            if_true,
            otherwise,
        }) => {
            let (condition, _) = eval(bump, predicate, scopes.clone())?;

            let (value, _) = condition.flatmap_boolean(spanned.span(), |boolean| {
                if boolean {
                    eval(bump, if_true, scopes.clone())
                } else {
                    eval(bump, otherwise, scopes.clone())
                }
            })?;

            (value, scopes)
        }
        Expr::Function(function) => {
            let cloned_scopes = scopes.clone();
            let f = move |callsite: Span, args: Vec<'v, Value<'v, 's>>| {
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

                    eval(bump, &function.body, body_scope)
                }
            };

            (Value::Function(Rc::new(f)), cloned_scopes)
        }
        Expr::Application(function_expr, args) => {
            let evaluated_args: Vec<'v, Value<'v, 's>> = args
                .iter()
                .map(|arg| eval(bump, arg, scopes.clone()).map(|(value, _)| value))
                .collect_in::<Result<_, Error>>(bump)?;

            let (function_value, _) = eval(bump, function_expr, scopes.clone())?;

            match function_value {
                Value::Function(function) => {
                    let (value, _) = function(spanned.span(), evaluated_args)?;
                    (value, scopes)
                }
                Value::BuiltinFunction(function) => {
                    let (value, _) = function(spanned.span(), evaluated_args)?;
                    (value, scopes)
                }
                _ => Err(Error::new(
                    ErrorKind::Expected(Type::Function, function_value),
                    function_expr.span(),
                ))?,
            }
        }
        Expr::Let(Let { assignments, body }) => {
            let new_scope = assignments.iter().try_fold(
                HashTrieMap::new(),
                |mut accumulated_scope, assignment| {
                    let (value, _) = eval(bump, &assignment.expr, scopes.clone())?;

                    accumulated_scope.insert_mut(assignment.identifier, value);
                    Ok(accumulated_scope)
                },
            )?;

            let (value, _) = eval(bump, body, scopes.push_front(new_scope))?;

            (value, scopes)
        }
        Expr::BinaryOp(left_expr, op, right_expr) => {
            let output = match op {
                BinaryOp::Numeric(numeric_op) => {
                    let operator = |x: f32, y: f32| match numeric_op {
                        NumericBinaryOp::Add => Ok(x + y),
                        NumericBinaryOp::Sub => Ok(x - y),
                        NumericBinaryOp::Mul => Ok(x * y),
                        NumericBinaryOp::Div => {
                            let dividend = x;
                            let divisor = y;

                            match dividend {
                                0.0 => {
                                    Err(Error::new(ErrorKind::DivisionByZero, right_expr.span()))
                                }
                                f32::INFINITY | f32::NEG_INFINITY => Err(Error::new(
                                    ErrorKind::DivisionByInfinity,
                                    right_expr.span(),
                                )),
                                _ => Ok(dividend / divisor),
                            }
                        }
                        NumericBinaryOp::Exp => {
                            let base = x;
                            let exponent = y;
                            let integer_part = exponent.trunc();

                            if integer_part == exponent {
                                Ok(base.powi(exponent as i32))
                            } else if integer_part == 0.0 && base < 0.0 {
                                Err(Error::new(
                                    ErrorKind::FractionalExponentWithNegativeBase {
                                        base,
                                        exponent,
                                    },
                                    spanned.span(),
                                ))
                            } else {
                                Ok(base.powf(exponent))
                            }
                        }
                    };

                    let (left, _) = eval(bump, left_expr, scopes.clone())?;

                    match left {
                        Value::Number(x) => {
                            let (right, _) = eval(bump, right_expr, scopes.clone())?;

                            match right {
                                Value::Number(y) => operator(x, y).map(Value::Number),

                                _ => Err(Error::new(
                                    ErrorKind::Expected(Type::Number, right),
                                    right_expr.span(),
                                )),
                            }
                        }
                        _ => Err(Error::new(
                            ErrorKind::Expected(Type::Number, left),
                            left_expr.span(),
                        )),
                    }?
                }
                BinaryOp::Comparison(comparison_op) => {
                    let (left, _) = eval(bump, left_expr, scopes.clone())?;
                    let (right, _) = eval(bump, right_expr, scopes.clone())?;

                    match (left, right) {
                        (Value::Number(x), Value::Number(y)) => {
                            Ok(Value::Boolean(compare(comparison_op, x, y)))
                        }

                        (Value::Boolean(x), Value::Boolean(y)) => {
                            Ok(Value::Boolean(compare(comparison_op, x, y)))
                        }

                        (
                            Value::Function(_) | Value::BuiltinFunction(_),
                            Value::Function(_) | Value::BuiltinFunction(_),
                        ) => Err(Error::new(
                            ErrorKind::CannotCompareFunctions,
                            spanned.span(),
                        )),

                        _ => Err(Error::new(
                            ErrorKind::ComparisonBinaryOperatorTypeMismatch { left, right },
                            spanned.span(),
                        )),
                    }?
                }
            };

            (output, scopes)
        }
        Expr::UnaryOp(op, expr) => match op {
            UnaryOpKind::Negate => {
                let (value, _) = eval(bump, expr, scopes.clone())?;
                let negated = value.map_number(expr.span(), ops::Neg::neg)?;
                (negated, scopes)
            }
        },
    })
}

fn compare<T: cmp::PartialEq + cmp::PartialOrd>(op: &ComparisonBinaryOp, x: T, y: T) -> bool {
    match op {
        ComparisonBinaryOp::Equals => x == y,
        ComparisonBinaryOp::NotEquals => x != y,
        ComparisonBinaryOp::LessThan => x < y,
        ComparisonBinaryOp::LessThanOrEquals => x <= y,
        ComparisonBinaryOp::GreaterThan => x > y,
        ComparisonBinaryOp::GreaterThanOrEquals => x >= y,
    }
}
