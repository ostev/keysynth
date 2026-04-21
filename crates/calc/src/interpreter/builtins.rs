use core::f32;

use alloc::vec::Vec;
use micromath::F32Ext;
use rpds::{HashTrieMap, ht_map};

use crate::{
    Error, ErrorKind, Value,
    interpreter::{EvalResult, Type},
    parser::ast::Span,
};

pub type BuiltinFunction = for<'a, 's> fn(Span, Vec<Value<'a, 's>>) -> EvalResult<'a, 's>;

#[derive(Clone, Copy)]
pub enum BuiltinValue {
    Number(f32),
    Function(BuiltinFunction),
}

pub fn get_builtin(name: &str) -> Option<BuiltinValue> {
    Some(match name {
        "e" => BuiltinValue::Number(f32::consts::E),
        "pi" => BuiltinValue::Number(f32::consts::PI),
        "tau" => BuiltinValue::Number(f32::consts::TAU),

        "half_pi" => BuiltinValue::Number(f32::consts::FRAC_PI_2),
        "third_pi" => BuiltinValue::Number(f32::consts::FRAC_PI_3),
        "quarter_pi" => BuiltinValue::Number(f32::consts::FRAC_PI_4),
        "sixth_pi" => BuiltinValue::Number(f32::consts::FRAC_PI_6),
        "eight_pi" => BuiltinValue::Number(f32::consts::FRAC_PI_8),

        "ln_2" => BuiltinValue::Number(f32::consts::LN_2),
        "ln_10" => BuiltinValue::Number(f32::consts::LN_10),
        "log2_e" => BuiltinValue::Number(f32::consts::LOG2_E),
        "log10_e" => BuiltinValue::Number(f32::consts::LOG10_E),

        "recip_pi" => BuiltinValue::Number(f32::consts::FRAC_1_PI),
        "recip_2pi" => BuiltinValue::Number(f32::consts::FRAC_2_PI),
        "recip_sqrt_pi" => BuiltinValue::Number(f32::consts::FRAC_1_SQRT_2),
        "recip_2sqrt_pi" => BuiltinValue::Number(f32::consts::FRAC_2_SQRT_PI),

        "sqrt_2" => BuiltinValue::Number(f32::consts::SQRT_2),
        "recip_sqrt_2" => BuiltinValue::Number(f32::consts::FRAC_1_SQRT_2),

        "sin" => BuiltinValue::Function(|callsite, args| {
            unary_numeric(callsite, args, |_, value| Ok(value.sin()))
        }),
        "cos" => BuiltinValue::Function(|callsite, args| {
            unary_numeric(callsite, args, |_, value| Ok(value.cos()))
        }),
        "tan" => BuiltinValue::Function(|callsite, args| {
            unary_numeric(callsite, args, |_, value| Ok(value.tan()))
        }),
        "asin" => BuiltinValue::Function(|callsite, args| {
            unary_numeric(callsite, args, inverse_trig(f32::asin))
        }),
        "acos" => BuiltinValue::Function(|callsite, args| {
            unary_numeric(callsite, args, inverse_trig(f32::acos))
        }),
        "atan" => BuiltinValue::Function(|callsite, args| {
            unary_numeric(callsite, args, inverse_trig(f32::atan))
        }),
        "sqrt" => {
            BuiltinValue::Function(|callsite, args| unary_numeric(callsite, args, sqrt_number))
        }
        "cbrt" => BuiltinValue::Function(|callsite, args| {
            unary_numeric(callsite, args, |_, value| Ok(cube_root(value)))
        }),
        "root" => BuiltinValue::Function(|callsite, args| binary_numeric(callsite, args, nth_root)),

        _ => return None,
    })
}

fn unary_numeric<'v, 's>(
    callsite: Span,
    args: Vec<Value<'v, 's>>,
    f: impl Fn(Span, f32) -> Result<f32, Error<'v, 's>>,
) -> EvalResult<'v, 's> {
    if args.len() == 1 {
        let value = match &args[0] {
            Value::Number(number) => f(callsite, *number).map(Value::Number),
            value => Err(Error::new(
                ErrorKind::Expected(Type::Number, value.clone()),
                callsite,
            )),
        }?;

        Ok((value, rpds::List::new()))
    } else {
        Err(Error::new(
            ErrorKind::IncorrectArity {
                expected: 1,
                received: args.len(),
            },
            callsite,
        ))
    }
}

fn binary_numeric<'a, 's>(
    callsite: Span,
    args: Vec<Value<'a, 's>>,
    f: impl Fn(Span, f32, f32) -> Result<f32, Error<'a, 's>>,
) -> EvalResult<'a, 's> {
    if args.len() == 2 {
        let value = match (&args[0], &args[1]) {
            (Value::Number(x), Value::Number(y)) => f(callsite, *x, *y).map(Value::Number),
            (Value::Number(_), y) => Err(Error::new(
                ErrorKind::Expected(Type::Number, y.clone()),
                callsite,
            )),
            (x, _) => Err(Error::new(
                ErrorKind::Expected(Type::Number, x.clone()),
                callsite,
            )),
        }?;

        Ok((value, rpds::List::new()))
    } else {
        Err(Error::new(
            ErrorKind::IncorrectArity {
                expected: 2,
                received: args.len(),
            },
            callsite,
        ))
    }
}

// Returns -1 raised to the power of the exponent
const fn negative_one_pow(exponent: i32) -> i32 {
    1 - ((exponent & 1) * 2)
}

fn cube_root(value: f32) -> f32 {
    let positive_root = value.abs().powf(1.0 / 3.0);
    let sign = (-1 as isize).pow(3);

    sign as f32 * positive_root
}

fn nth_root<'v, 's>(span: Span, n: f32, value: f32) -> Result<f32, Error<'v, 's>> {
    if n >= 0.0 {
        if value >= 0.0 {
            Ok(value.powf(1.0 / n))
        } else {
            let truncated_n = n.trunc();
            let integer_n = truncated_n as i32;
            if truncated_n == n && integer_n % 2 != 0 {
                // The root is an odd, whole number, so we handle the sign
                // ourselves
                let positive_root = value.abs().powf(1.0 / n);
                let sign = negative_one_pow(integer_n);

                Ok(sign as f32 * positive_root)
            } else {
                Err(Error::new(
                    ErrorKind::ArbitraryRootThatIsNotOddOfNegativeValue { root: n, value },
                    span,
                ))
            }
        }
    } else {
        Err(Error::new(ErrorKind::NegativeRoot { root: n, value }, span))
    }
}

fn sqrt_number<'v, 's>(span: Span, value: f32) -> Result<f32, Error<'v, 's>> {
    if value < 0.0 {
        Err(Error::new(
            ErrorKind::EvenRootOfNegativeValue { root: 2.0, value },
            span,
        ))
    } else {
        Ok(value.sqrt())
    }
}

/// Transforms an inverse trig function to a checked version
/// that returns an error if the provided value is outside its
/// domain instead of returning `NaN`.
fn inverse_trig<'v, 's>(
    f: impl Fn(f32) -> f32,
) -> impl Fn(Span, f32) -> Result<f32, Error<'v, 's>> {
    move |span, value| {
        if value >= -1.0 && value <= 1.0 {
            Ok(f(value))
        } else {
            Err(Error::new(
                ErrorKind::OutsideFunctionDomain {
                    domain: (-1.0, 1.0),
                    value,
                },
                span,
            ))
        }
    }
}
