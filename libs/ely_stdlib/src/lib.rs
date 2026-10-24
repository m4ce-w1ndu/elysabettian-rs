use std::collections::HashMap;

use ely_core::interner::StringInterner;
use ely_core::value::Value;

pub mod io;
pub mod math;

/// Function pointer signature for native standard library functions.
pub type NativeFn = fn(arg_count: u8, args: &[Value]) -> Value;

/// Represents a named standard library module containing native functions and constants.
pub struct NativeModule {
    pub name: &'static str,
    pub functions: Vec<(&'static str, NativeFn)>,
    pub constants: Vec<(&'static str, Value)>,
}

impl NativeModule {
    /// Creates a new module with the specified name.
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            functions: Vec::new(),
            constants: Vec::new(),
        }
    }

    /// Registers a native function in the module.
    pub fn add_fn(&mut self, name: &'static str, func: NativeFn) -> &mut Self {
        self.functions.push((name, func));
        self
    }

    /// Registers a constant value in the module.
    pub fn add_const(&mut self, name: &'static str, val: Value) -> &mut Self {
        self.constants.push((name, val));
        self
    }
}

/// Returns the complete list of standard library modules.
pub fn get_standard_modules() -> Vec<NativeModule> {
    vec![
        math::module(),
        io::module(),
    ]
}

/// Registers all functions and constants from a module into VM native tables and globals.
pub fn register_module(
    module: &NativeModule,
    natives: &mut HashMap<u32, NativeFn>,
    globals: &mut HashMap<u32, Value>,
    interner: &mut StringInterner,
) {
    for &(name, func) in &module.functions {
        let name_id = interner.intern(name);
        natives.insert(name_id, func);
        globals.insert(name_id, Value::obj(name_id as u64));
    }
    for &(name, val) in &module.constants {
        let name_id = interner.intern(name);
        globals.insert(name_id, val);
    }
}

/// Registers all standard library modules into VM native tables and globals.
pub fn register_all(
    natives: &mut HashMap<u32, NativeFn>,
    globals: &mut HashMap<u32, Value>,
    interner: &mut StringInterner,
) {
    for module in get_standard_modules() {
        register_module(&module, natives, globals, interner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_math_sqrt_and_pow() {
        let val_16 = Value::number(16.0);
        let res = math::sqrt(1, &[val_16]);
        assert_eq!(res, Value::number(4.0));

        let val_2 = Value::number(2.0);
        let val_3 = Value::number(3.0);
        let res_pow = math::pow(2, &[val_2, val_3]);
        assert_eq!(res_pow, Value::number(8.0));
    }

    #[test]
    fn test_io_clock() {
        let t = io::clock(0, &[]);
        assert!(t.is_number());
        assert!(t.as_number() > 0.0);
    }
}
