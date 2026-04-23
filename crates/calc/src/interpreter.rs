use core::{cmp, f32, fmt, ops};

use alloc::{format, rc::Rc, string::ToString, vec, vec::Vec};
use bumpalo::{Bump, boxed::Box};
use codespan_reporting::diagnostic::{Diagnostic, Label};
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

/// Represents a calculator value at runtime, which is either a number,
/// a boolean, a function or a builtin.
#[derive(Clone)]
pub enum Value<'a, 's> {
    Number(f32),
    Boolean(bool),
    Function(FunctionValue<'a, 's>),
    BuiltinFunction(BuiltinFunction),
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

#[derive(Debug)]
pub struct Error<'a, 's> {
    pub kind: ErrorKind<'a, 's>,
    pub span: (usize, usize),
}

impl<'a, 's> Error<'a, 's> {
    pub const fn new(kind: ErrorKind<'a, 's>, span: (usize, usize)) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug)]
pub enum ErrorKind<'a, 's> {
    IdentifierNotInScope(&'s str),
    Expected(Type, Value<'a, 's>),
    CannotCompareFunctions {
        lhs: Span,
        rhs: Span,
    },
    ComparisonBinaryOperatorTypeMismatch {
        lhs: (Span, Value<'a, 's>),
        rhs: (Span, Value<'a, 's>),
    },
    IncorrectArity {
        function_span: Span,
        expected: usize,
        received: usize,
    },
    DivisionByZero(Span),
    DivisionByInfinity(Span),
    FractionalExponentWithNegativeBase {
        base: f32,
        exponent: f32,
    },
    OutsideFunctionDomain {
        domain: (f32, f32),
        value: f32,
    },
    RootThatIsNotOddOfNegativeValue {
        root: f32,
        value: f32,
    },
    InvalidRoot {
        root: f32,
        value: f32,
    },
}

/// Converts an error into a printable diagnostic
impl<'a, 's> Into<Diagnostic<()>> for Error<'a, 's> {
    fn into(self) -> Diagnostic<()> {
        let (start, end) = self.span;
        // Span is inclusive, the range is not
        let range = start..(end + 1);

        let primary = Label::primary((), range.clone());
        let secondary = Label::secondary((), range);

        match self.kind {
            ErrorKind::IdentifierNotInScope(identifier) => Diagnostic::error()
                .with_code("E001")
                .with_label(primary.with_message("this variable is not in scope"))
                .with_note(format!(
                    "I can't find a variable or function in scope called {identifier}! \
Make sure it's defined and you haven't made a typo."
                )),

            ErrorKind::Expected(expected_type, value) => Diagnostic::error()
                .with_code("E002")
                .with_labels(vec![
                    primary.with_message(format!(
                        "expected this value to be of type {}",
                        expected_type
                    )),
                    secondary.with_message(format!(
                        "this value, {value}, is of type {}",
                        value.as_type()
                    )),
                ])
                .with_note(format!(
                    "I expected a value {value} of type {} here, but got a {} instead. \
You might want to investigate the series of operations that produced {value}.",
                    expected_type,
                    value.as_type()
                )),
            ErrorKind::CannotCompareFunctions { lhs, rhs } => Diagnostic::error()
                .with_code("E003")
                .with_labels(vec![
                    primary
                        .with_message("both values on either side of this operand are functions"),
                    Label::secondary((), lhs.0..(lhs.1 + 1))
                        .with_message("this value is a function"),
                    Label::secondary((), rhs.0..(rhs.1 + 1))
                        .with_message("this value is also a function"),
                ])
                .with_note("I can't compare functions, since there's no meaningful way to do it."),
            ErrorKind::ComparisonBinaryOperatorTypeMismatch {
                lhs: (lhs_span, lhs),
                rhs: (rhs_span, rhs),
            } => Diagnostic::error()
                .with_code("E004")
                .with_labels(vec![
                    primary.with_message(
                        "the values on either side of this operand have different types",
                    ),
                    Label::secondary((), lhs_span.0..(lhs_span.1 + 1))
                        .with_message(format!("this value, {lhs}, is a {}", lhs.as_type())),
                    Label::secondary((), rhs_span.0..(rhs_span.1 + 1))
                        .with_message(format!("this value, {rhs}, is a {}", rhs.as_type())),
                ])
                .with_note("I can't compare two values of different types. If you want to convert them, use `bool` or `num`."),
            ErrorKind::IncorrectArity { function_span, expected, received } => Diagnostic::error()
                .with_code("E005")
                .with_labels(vec![
                    primary.with_message(
                        format!("you've passed {received} arguments to a function that expects {expected}")
                    ),
                    Label::secondary((), function_span.0..(function_span.1 + 1))
                        .with_message(format!("this function expects {expected} arguments"))
                ])
                .with_note("This function is of a different arity to what you've called it as."),
            ErrorKind::DivisionByZero(span) => Diagnostic::error()
                .with_code("E006")
                .with_labels(
                    vec![
                        primary.with_message("this expression divides by zero"),
                        Label::secondary((), span.0..(span.1 + 1)).with_message("this value is zero")
                    ]
                )
                .with_note("I can't divide by zero as it is undefined."),
            ErrorKind::DivisionByInfinity(span) => Diagnostic::error()
                .with_code("E007")
                .with_labels(
                    vec![
                        primary.with_message("this expression divides by infinity"),
                        Label::secondary((), span.0..(span.1 + 1)).with_message("this value is infinite")
                    ]
                )
                .with_note(
                    "I can't divide by infinity as it is undefined.",
                ),
            ErrorKind::FractionalExponentWithNegativeBase {
                base,
                exponent,
            } => {
                Diagnostic::error()
                    .with_code("E008")
                    .with_labels(vec![
                        primary.with_message(
                            format!("this expression raises a negative base, {base}, to a fractional exponent, {exponent}"),
                        ),
                    ])
                    .with_note(format!(
                        "I can't compute {base}^{exponent} as a real number. If you want to find the nth \
root where n is odd and so {base} has a real root, use the `root` function.",
                    ))
            }
            ErrorKind::OutsideFunctionDomain {
                domain,
                value,
            } => Diagnostic::error()
                .with_code("E009")
                .with_labels(vec![
                    primary.with_message(
                        format!(
                            "this value, {value}, is outside the domain [{}, {}]",
                            domain.0,
                            domain.1
                        )
                    ),
                ])
                .with_note(format!(
                    "This function only accepts values in the domain [{}, {}], but got {value}. This \
means that {value} does not satisfy the condition {} <= {value} <= {}",
                    domain.0, domain.1, domain.0, domain.1
                )),
            ErrorKind::RootThatIsNotOddOfNegativeValue{  root, value } => {
                let root_name = match root {
                    2.0 => "square".to_string(),
                    3.0 => "cube".to_string(),
                    _ => {
                        let truncated = root.trunc();
                        if truncated == root {
                            // It's a whole number
                            format!("{:.0}nth" ,truncated)
                        } else {
                            format!("{root}-nth")
                        }
                    }
                };
                Diagnostic::error()
                .with_code("E010")
                .with_labels(vec![
                    primary.with_message(format!(
                        "this takes the {root_name} of {value}, which is negative"
                    )),
                ])
                .with_note(format!(
                    "The {root_name} root of {value} is not real, so I can't compute it.",
                ))},
   
            ErrorKind::InvalidRoot { root, value } => {
                let reason = if root < 0.0 {
                            "negative"
                        } else if root == 0.0 {
                            "zero"
                        } else {
                            "invalid"
                        };

                Diagnostic::error()
                .with_code("E012")
                .with_labels(vec![
                    primary.with_message(
                        format!("this root, {root}, is {reason}")
                    ),
                ])
                .with_note(format!(
                    "The {root}-th root of {value} is not real, so I can't compute it.",
                ))},
        }
    }
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
                    let (value, _) = eval(&assignment.expr, scopes.clone())?;

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

                            match dividend {
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
                                        span: left_expr.span(),
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
