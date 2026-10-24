use ely_core::value::Value;
use crate::NativeModule;

/// Constructs the `math` native standard library module.
pub fn module() -> NativeModule {
    let mut module = NativeModule::new("math");
    module
        .add_fn("sqrt", sqrt)
        .add_fn("pow", pow)
        .add_fn("abs", abs)
        .add_fn("floor", floor)
        .add_fn("ceil", ceil)
        .add_fn("round", round)
        .add_fn("sin", sin)
        .add_fn("cos", cos)
        .add_fn("tan", tan)
        .add_fn("log", log)
        .add_fn("min", min)
        .add_fn("max", max)
        .add_const("PI", Value::number(std::f64::consts::PI))
        .add_const("E", Value::number(std::f64::consts::E));
    module
}

/// Returns the square root of a number: `sqrt(x)`
pub fn sqrt(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().sqrt())
}

/// Returns the base to the exponent power: `pow(base, exp)`
pub fn pow(argc: u8, args: &[Value]) -> Value {
    if argc < 2 || !args[0].is_number() || !args[1].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().powf(args[1].as_number()))
}

/// Returns the absolute value of a number: `abs(x)`
pub fn abs(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().abs())
}

/// Rounds a number downward: `floor(x)`
pub fn floor(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().floor())
}

/// Rounds a number upward: `ceil(x)`
pub fn ceil(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().ceil())
}

/// Rounds a number to the nearest integer: `round(x)`
pub fn round(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().round())
}

/// Returns the sine of a number (radians): `sin(x)`
pub fn sin(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().sin())
}

/// Returns the cosine of a number (radians): `cos(x)`
pub fn cos(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().cos())
}

/// Returns the tangent of a number (radians): `tan(x)`
pub fn tan(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().tan())
}

/// Returns the natural logarithm of a number: `log(x)`
pub fn log(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().ln())
}

/// Returns the minimum of two numbers: `min(a, b)`
pub fn min(argc: u8, args: &[Value]) -> Value {
    if argc < 2 || !args[0].is_number() || !args[1].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().min(args[1].as_number()))
}

/// Returns the maximum of two numbers: `max(a, b)`
pub fn max(argc: u8, args: &[Value]) -> Value {
    if argc < 2 || !args[0].is_number() || !args[1].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().max(args[1].as_number()))
}
