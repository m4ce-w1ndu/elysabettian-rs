use std::cell::RefCell;
use std::collections::HashMap;

use ely_core::value::Value;

// Thread-local runtime context for JIT native execution
thread_local! {
    static JIT_GLOBALS: RefCell<HashMap<u32, Value>> = RefCell::new(HashMap::new());
    static JIT_STRINGS: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static JIT_FUNCTIONS: RefCell<HashMap<usize, *const u8>> = RefCell::new(HashMap::new());
}

/// Registers a compiled function pointer in the JIT runtime table.
pub fn register_jit_function(func_idx: usize, ptr: *const u8) {
    JIT_FUNCTIONS.with(|fns| {
        fns.borrow_mut().insert(func_idx, ptr);
    });
}

/// Registers string constants for the current JIT compilation session.
pub fn register_jit_strings(strings: &[String]) {
    JIT_STRINGS.with(|s| {
        *s.borrow_mut() = strings.to_vec();
    });
}

/// Appends string constants to the current JIT compilation session and returns the starting index.
pub fn append_jit_strings(strings: &[String]) -> usize {
    JIT_STRINGS.with(|s| {
        let mut borrowed = s.borrow_mut();
        let base_idx = borrowed.len();
        borrowed.extend(strings.iter().cloned());
        base_idx
    })
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

/// Dynamic call trampoline for 0-argument JIT functions.
pub extern "C" fn ely_jit_call_0(callee_bits: u64) -> f64 {
    let callee = Value(callee_bits);
    let func_idx = callee.as_obj() as usize;
    let code_ptr = JIT_FUNCTIONS.with(|fns| fns.borrow().get(&func_idx).copied());
    if let Some(ptr) = code_ptr {
        let func: extern "C" fn() -> f64 = unsafe { std::mem::transmute(ptr) };
        func()
    } else {
        0.0
    }
}

/// Dynamic call trampoline for 1-argument JIT functions.
pub extern "C" fn ely_jit_call_1(callee_bits: u64, arg0: f64) -> f64 {
    let callee = Value(callee_bits);
    let func_idx = callee.as_obj() as usize;
    let code_ptr = JIT_FUNCTIONS.with(|fns| fns.borrow().get(&func_idx).copied());
    if let Some(ptr) = code_ptr {
        let func: extern "C" fn(f64) -> f64 = unsafe { std::mem::transmute(ptr) };
        func(arg0)
    } else {
        0.0
    }
}

/// Dynamic call trampoline for 2-argument JIT functions.
pub extern "C" fn ely_jit_call_2(callee_bits: u64, arg0: f64, arg1: f64) -> f64 {
    let callee = Value(callee_bits);
    let func_idx = callee.as_obj() as usize;
    let code_ptr = JIT_FUNCTIONS.with(|fns| fns.borrow().get(&func_idx).copied());
    if let Some(ptr) = code_ptr {
        let func: extern "C" fn(f64, f64) -> f64 = unsafe { std::mem::transmute(ptr) };
        func(arg0, arg1)
    } else {
        0.0
    }
}
