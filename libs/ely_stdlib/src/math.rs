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
        .add_fn("trunc", trunc)
        .add_fn("sin", sin)
        .add_fn("cos", cos)
        .add_fn("tan", tan)
        .add_fn("asin", asin)
        .add_fn("acos", acos)
        .add_fn("atan", atan)
        .add_fn("atan2", atan2)
        .add_fn("exp", exp)
        .add_fn("log", log)
        .add_fn("log10", log10)
        .add_fn("log2", log2)
        .add_fn("hypot", hypot)
        .add_fn("min", min)
        .add_fn("max", max)
        .add_const("PI", Value::number(std::f64::consts::PI))
        .add_const("TAU", Value::number(std::f64::consts::TAU))
        .add_const("E", Value::number(std::f64::consts::E))
        .add_const("INFINITY", Value::number(f64::INFINITY));
    module
}

/// Returns the square root of a number: `sqrt(x)`
pub fn sqrt(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().sqrt())
}

/// Returns the base raised to the exponent power: `pow(base, exp)`
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

/// Rounds a number downward to the nearest integer: `floor(x)`
pub fn floor(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().floor())
}

/// Rounds a number upward to the nearest integer: `ceil(x)`
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

/// Returns the integer part of a number by dropping fractional digits: `trunc(x)`
pub fn trunc(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().trunc())
}

/// Returns the sine of a number in radians: `sin(x)`
pub fn sin(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().sin())
}

/// Returns the cosine of a number in radians: `cos(x)`
pub fn cos(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().cos())
}

/// Returns the tangent of a number in radians: `tan(x)`
pub fn tan(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().tan())
}

/// Returns the arcsine (inverse sine) in radians: `asin(x)`
pub fn asin(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().asin())
}

/// Returns the arccosine (inverse cosine) in radians: `acos(x)`
pub fn acos(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().acos())
}

/// Returns the arctangent (inverse tangent) in radians: `atan(x)`
pub fn atan(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().atan())
}

/// Returns the four-quadrant arctangent of y and x in radians: `atan2(y, x)`
pub fn atan2(argc: u8, args: &[Value]) -> Value {
    if argc < 2 || !args[0].is_number() || !args[1].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().atan2(args[1].as_number()))
}

/// Returns Euler's number `e` raised to the given power: `exp(x)`
pub fn exp(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().exp())
}

/// Returns the natural logarithm (base e) of a number: `log(x)`
pub fn log(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().ln())
}

/// Returns the base-10 logarithm of a number: `log10(x)`
pub fn log10(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().log10())
}

/// Returns the base-2 logarithm of a number: `log2(x)`
pub fn log2(argc: u8, args: &[Value]) -> Value {
    if argc < 1 || !args[0].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().log2())
}

/// Returns the Euclidean distance `sqrt(x*x + y*y)`: `hypot(x, y)`
pub fn hypot(argc: u8, args: &[Value]) -> Value {
    if argc < 2 || !args[0].is_number() || !args[1].is_number() {
        return Value::null();
    }
    Value::number(args[0].as_number().hypot(args[1].as_number()))
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
