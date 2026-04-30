use core::{cmp, f32, fmt, ops};

use alloc::{format, rc::Rc, string::ToString, vec, vec::Vec};
use bumpalo::{Bump, boxed::Box};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use micromath::F32Ext;
use rpds::{HashTrieMap, List};

use crate::{
    interpreter::{
        builtins::{BuiltinFunction, BuiltinValue, get_builtin},
        diagnostic::{Error, ErrorKind},
    },
    parser::ast::{
        BinaryOp, Branch, ComparisonBinaryOp, Expr, Let, Literal, NumericBinaryOp, Span, Spanned,
        UnaryOpKind,
    },
};

pub mod builtins;
pub mod diagnostic;

/// Represents a calculator value at runtime, which is either a number,
/// a boolean, a function or a builtin.
#[derive(Clone)]
pub enum Value<'a, 's> {
    Number(f32),
    Boolean(bool),
    Function(FunctionValue<'a, 's>),
    BuiltinFunction(BuiltinFunction),
}

impl<'a, 's> Value<'a, 's> {
    pub fn to_number(self) -> Option<f32> {
        match self {
            Value::Number(num) => Some(num),
            _ => None,
        }
    }
    pub fn to_boolean(self) -> Option<bool> {
        match self {
            Value::Boolean(bool) => Some(bool),
            _ => None,
        }
    }

    pub fn as_concrete(&self) -> Option<OwnedValue> {
        match self {
            Value::Boolean(bool) => Some(OwnedValue::Boolean(*bool)),
            Value::Number(num) => Some(OwnedValue::Number(*num)),
            _ => None,
        }
    }
}

pub enum OwnedValue {
    Number(f32),
    Boolean(bool),
    Function,
}

/// A closure (function value) is allocated as a reference-counted value on the heap and
/// contains references to its surrounding scope. The scope will only be dropped when its
/// reference count is zero, so it's important that drop will be run on [`FunctionValue`]
/// to prevent memory leaks. It takes two [`Span`]s, the callsite and the function expression
/// (`f` in `f(x)` for example).
pub type FunctionValue<'a, 's> =
    Rc<dyn Fn(Span, Span, Vec<Value<'a, 's>>) -> EvalResult<'a, 's> + 'a>;

impl<'a, 's> fmt::Debug for Value<'a, 's> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(number) => write!(f, "{}", number),
            Self::Boolean(boolean) => write!(f, "{}", boolean),
            Self::Function(_) => write!(f, "{}", "<fn>"),
            Self::BuiltinFunction(_) => write!(f, "{}", "<builtin>"),
        }
    }
}

impl<'a, 's> fmt::Display for Value<'a, 's> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl<'a, 's> From<BuiltinValue> for Value<'a, 's> {
    fn from(value: BuiltinValue) -> Self {
        match value {
            BuiltinValue::Number(num) => Value::Number(num),
            BuiltinValue::Function(f) => Value::BuiltinFunction(f),
        }
    }
}

impl<'a, 's> Value<'a, 's> {
    pub fn map_number(self, span: Span, f: impl Fn(f32) -> f32) -> Result<Self, Error<'a, 's>> {
        match self {
            Value::Number(number) => Ok(Value::Number(f(number))),
            _ => Err(Error::new(ErrorKind::Expected(Type::Number, self), span)),
        }
    }
    pub fn flatmap_number(
        self,
        span: Span,
        f: impl Fn(f32) -> EvalResult<'a, 's>,
    ) -> EvalResult<'a, 's> {
        match self {
            Value::Number(number) => f(number),
            _ => Err(Error::new(ErrorKind::Expected(Type::Number, self), span)),
        }
    }

    pub fn flatmap_boolean(
        self,
        span: Span,
        f: impl Fn(bool) -> EvalResult<'a, 's>,
    ) -> EvalResult<'a, 's> {
        match self {
            Value::Boolean(boolean) => f(boolean),
            _ => Err(Error::new(ErrorKind::Expected(Type::Boolean, self), span)),
        }
    }

    pub fn map_boolean(self, span: Span, f: &impl Fn(bool) -> bool) -> Result<Self, Error<'a, 's>> {
        match self {
            Value::Boolean(boolean) => Ok(Value::Boolean(f(boolean))),
            _ => Err(Error::new(ErrorKind::Expected(Type::Boolean, self), span)),
        }
    }

    /// Returns the [`Type`] of the value
    pub fn as_type(&self) -> Type {
        match self {
            Value::Boolean(_) => Type::Boolean,
            Value::Number(_) => Type::Number,
            Value::Function(_) => Type::Function,
            Value::BuiltinFunction(_) => Type::Function,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Type {
    Boolean,
    Number,
    Function,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Type::Boolean => "boolean",
                Type::Number => "number",
                Type::Function => "function",
            }
        )
    }
}

/// A scope is defined as a stack of mappings between a name and a potentially evaluated value.
/// When the code contains an identifier, the interpreter checks each scope in the stack
/// (starting at the front) to see if it contains the identifier. If the identifier is a thunk
/// (i.e. has not yet been evaluated because of assignment ordering), the assignment that
/// references it will become a thunk.
pub type Scopes<'a, 's> = rpds::List<rpds::HashTrieMap<&'s str, Value<'a, 's>>>;

/// Represents the result of an interpreted expression, along with its scope.
pub type EvalResult<'a, 's> = Result<(Value<'a, 's>, Scopes<'a, 's>), Error<'a, 's>>;

pub fn eval_root<'a, 's>(spanned: &'a Spanned<'a, 's>) -> Result<Value<'a, 's>, Error<'a, 's>> {
    let (value, _scopes) = eval(spanned, List::new())?;
    Ok(value)
}

pub fn eval<'a, 's>(spanned: &'a Spanned<'a, 's>, scopes: Scopes<'a, 's>) -> EvalResult<'a, 's> {
    Ok(match &spanned.expr {
        Expr::Identifier(identifier) => {
            let value = match scopes
                .iter()
                .find_map(|scope| scope.get(identifier).cloned())
            {
                Some(defined_value) => Some(defined_value),
                None => get_builtin(*identifier).map(Into::into),
            }
            .ok_or(Error::new(
                ErrorKind::IdentifierNotInScope(*identifier),
                spanned.span(),
            ))?;

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
            let (condition, _) = eval(predicate, scopes.clone())?;

            let (value, _) = condition.flatmap_boolean(spanned.span(), |boolean| {
                if boolean {
                    eval(if_true, scopes.clone())
                } else {
                    eval(otherwise, scopes.clone())
                }
            })?;

            (value, scopes)
        }
        Expr::Function(function) => {
            let cloned_scopes = scopes.clone();
            let f = move |callsite: Span, function_span: Span, args: Vec<Value<'a, 's>>| {
                if args.len() != function.args.len() {
                    Err(Error::new(
                        ErrorKind::IncorrectArity {
                            function_span,
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

                    eval(&function.body, body_scope)
                }
            };

            (Value::Function(Rc::new(f)), cloned_scopes)
        }
        Expr::Application(function_expr, args) => {
            let evaluated_args: Vec<Value<'a, 's>> = args
                .iter()
                .map(|arg| eval(arg, scopes.clone()).map(|(value, _)| value))
                .collect::<Result<Vec<_>, Error>>()?;

            let (function_value, _) = eval(function_expr, scopes.clone())?;

            match function_value {
                Value::Function(function) => {
                    let (value, _) =
                        function(spanned.span(), function_expr.span(), evaluated_args)?;
                    (value, scopes)
                }
                Value::BuiltinFunction(function) => {
                    let (value, _) =
                        function(spanned.span(), function_expr.span(), evaluated_args)?;
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
                    let (value, _) = eval(
                        &assignment.expr,
                        scopes.push_front(accumulated_scope.clone()),
                    )?;

                    accumulated_scope.insert_mut(assignment.identifier, value);
                    Ok(accumulated_scope)
                },
            )?;

            let (value, _) = eval(body, scopes.push_front(new_scope))?;

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

                            match divisor {
                                0.0 => Err(Error::new(
                                    ErrorKind::DivisionByZero(right_expr.span()),
                                    right_expr.span(),
                                )),
                                f32::INFINITY | f32::NEG_INFINITY => Err(Error::new(
                                    ErrorKind::DivisionByInfinity(right_expr.span()),
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

                    let (left, _) = eval(left_expr, scopes.clone())?;

                    match left {
                        Value::Number(x) => {
                            let (right, _) = eval(right_expr, scopes.clone())?;

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
                    let (left, _) = eval(left_expr, scopes.clone())?;
                    let (right, _) = eval(right_expr, scopes.clone())?;

                    match (&left, &right) {
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
                            ErrorKind::CannotCompareFunctions {
                                lhs: left_expr.span(),
                                rhs: right_expr.span(),
                            },
                            spanned.span(),
                        )),

                        _ => Err(Error::new(
                            ErrorKind::ComparisonBinaryOperatorTypeMismatch {
                                lhs: (left_expr.span(), left),
                                rhs: (right_expr.span(), right),
                            },
                            spanned.span(),
                        )),
                    }?
                }
            };

            (output, scopes)
        }
        Expr::UnaryOp(op, expr) => match op {
            UnaryOpKind::Negate => {
                let (value, _) = eval(expr, scopes.clone())?;
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
