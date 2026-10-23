use std::cell::RefCell;
use std::collections::HashMap;

use ely_core::value::Value;

// Thread-local runtime context for JIT native execution
thread_local! {
    static JIT_GLOBALS: RefCell<HashMap<u32, Value>> = RefCell::new(HashMap::new());
    static JIT_STRINGS: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

/// Registers string constants for the current JIT compilation session.
pub fn register_jit_strings(strings: &[String]) {
    JIT_STRINGS.with(|s| {
        *s.borrow_mut() = strings.to_vec();
    });
}

/// Appends string constants to the current JIT compilation session.
pub fn append_jit_strings(strings: &[String]) {
    JIT_STRINGS.with(|s| {
        s.borrow_mut().extend(strings.iter().cloned());
    });
}

/// Sets or defines a global variable in the JIT runtime state.
pub extern "C" fn ely_jit_set_global(id: u32, val_bits: u64) {
    JIT_GLOBALS.with(|globals| {
        globals.borrow_mut().insert(id, Value(val_bits));
    });
}

/// Retrieves a global variable from the JIT runtime state.
pub extern "C" fn ely_jit_get_global(id: u32) -> u64 {
    JIT_GLOBALS.with(|globals| {
        globals
            .borrow()
            .get(&id)
            .copied()
            .unwrap_or_else(Value::null)
            .0
    })
}

/// Formats and prints a NaN-boxed Value to standard output.
pub extern "C" fn ely_jit_print_value(val_bits: u64) {
    let value = Value(val_bits);
    if value.is_number() {
        println!("{}", value.as_number());
    } else if value.is_null() {
        println!("null");
    } else if value.is_bool() {
        println!("{}", value.as_bool());
    } else if value.is_obj() {
        let string_idx = value.as_obj() as usize;
        let printed = JIT_STRINGS.with(|strings| {
            if let Some(s) = strings.borrow().get(string_idx) {
                println!("{}", s);
                true
            } else {
                false
            }
        });

        if !printed {
            println!("{}", value);
        }
    } else {
        println!("{}", value);
    }
}
