use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use ely_core::value::Value;
use crate::NativeModule;

/// Constructs the `io` native standard library module.
pub fn module() -> NativeModule {
    let mut module = NativeModule::new("io");
    module
        .add_fn("clock", clock)
        .add_fn("time", clock)
        .add_fn("read", read);
    module
}

/// Returns the current high-resolution timestamp in seconds since the UNIX epoch: `clock()`
pub fn clock(_argc: u8, _args: &[Value]) -> Value {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    Value::number(now.as_secs_f64())
}

/// Reads a line from standard input, parsing it as a number if possible: `read()`
pub fn read(_argc: u8, _args: &[Value]) -> Value {
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim();
        if let Ok(num) = trimmed.parse::<f64>() {
            return Value::number(num);
        }
    }
    Value::null()
}
