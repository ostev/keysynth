use alloc::{format, string::ToString, vec, vec::{Vec}};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use micromath::F32Ext;

use crate::{interpreter::{Type, Value}, parser::ast::Span};


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
