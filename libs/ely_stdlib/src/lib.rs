use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr};
use std::path::{Path, PathBuf};

use ely_core::interner::StringInterner;
use ely_core::value::Value;

pub mod io;
pub mod math;

/// Function pointer signature for internal Rust native functions.
pub type NativeFn = fn(arg_count: u8, args: &[Value]) -> Value;

/// C-ABI compatible function pointer signature for foreign dynamically loaded functions (.so / .dylib).
pub type ExternNativeFn = unsafe extern "C" fn(arg_count: u8, args: *const Value) -> Value;

/// Unified callable representation for either an internal Rust native or an external C-ABI function.
#[derive(Clone, Copy, Debug)]
pub enum CallableNative {
    Rust(NativeFn),
    C(ExternNativeFn),
}

impl From<NativeFn> for CallableNative {
    fn from(f: NativeFn) -> Self {
        CallableNative::Rust(f)
    }
}

impl From<ExternNativeFn> for CallableNative {
    fn from(f: ExternNativeFn) -> Self {
        CallableNative::C(f)
    }
}

impl CallableNative {
    /// Invokes the native function with the given arguments slice.
    ///
    /// # Safety
    /// For C-ABI functions, the underlying pointer expects `arg_count` valid `Value` items.
    #[inline]
    pub unsafe fn invoke(&self, arg_count: u8, args: &[Value]) -> Value {
        match self {
            CallableNative::Rust(f) => f(arg_count, args),
            CallableNative::C(f) => f(arg_count, args.as_ptr()),
        }
    }
}

/// C-compatible API passed to foreign shared libraries upon module initialization.
#[repr(C)]
pub struct ElyApi {
    pub ctx: *mut c_void,
    pub register_fn:
        unsafe extern "C" fn(ctx: *mut c_void, name: *const c_char, func: ExternNativeFn),
    pub register_number: unsafe extern "C" fn(ctx: *mut c_void, name: *const c_char, val: f64),
}

/// Internal context structure passed to C registration trampolines.
struct LoadContext<'a> {
    natives: &'a mut HashMap<u32, CallableNative>,
    globals: &'a mut HashMap<u32, Value>,
    interner: &'a mut StringInterner,
}

unsafe extern "C" fn c_register_fn(ctx: *mut c_void, name: *const c_char, func: ExternNativeFn) {
    if ctx.is_null() || name.is_null() {
        return;
    }
    let load_ctx = &mut *(ctx as *mut LoadContext);
    let c_str = CStr::from_ptr(name);
    if let Ok(name_str) = c_str.to_str() {
        let id = load_ctx.interner.intern(name_str);
        load_ctx.natives.insert(id, CallableNative::C(func));
        load_ctx.globals.insert(id, Value::obj(id as u64));
    }
}

unsafe extern "C" fn c_register_number(ctx: *mut c_void, name: *const c_char, val: f64) {
    if ctx.is_null() || name.is_null() {
        return;
    }
    let load_ctx = &mut *(ctx as *mut LoadContext);
    let c_str = CStr::from_ptr(name);
    if let Ok(name_str) = c_str.to_str() {
        let id = load_ctx.interner.intern(name_str);
        load_ctx.globals.insert(id, Value::number(val));
    }
}

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

/// Module registry managing both built-in standard modules and dynamically loaded `.so` libraries.
pub struct ModuleRegistry {
    builtin_modules: HashMap<String, NativeModule>,
    search_paths: Vec<PathBuf>,
    loaded_libraries: Vec<libloading::Library>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleRegistry {
    /// Creates a new `ModuleRegistry` pre-populated with standard built-in modules (`math`, `io`, `stdio`).
    pub fn new() -> Self {
        let mut reg = Self {
            builtin_modules: HashMap::new(),
            search_paths: vec![
                PathBuf::from("./modules"),
                PathBuf::from("."),
                PathBuf::from("/usr/local/lib/ely"),
            ],
            loaded_libraries: Vec::new(),
        };

        reg.register_builtin(math::module());
        reg.register_builtin(io::module());
        // "stdio" alias for "io"
        let mut stdio = io::module();
        stdio.name = "stdio";
        reg.register_builtin(stdio);

        reg
    }

    /// Adds a directory path to search when dynamically loading external `.so` modules.
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) {
        self.search_paths.push(path.as_ref().to_path_buf());
    }

    /// Registers a built-in module into the registry.
    pub fn register_builtin(&mut self, module: NativeModule) {
        self.builtin_modules.insert(module.name.to_string(), module);
    }

    /// Loads a module by name: first checks built-in modules, then searches for an external `.so` library.
    pub fn load(
        &mut self,
        name: &str,
        natives: &mut HashMap<u32, CallableNative>,
        globals: &mut HashMap<u32, Value>,
        interner: &mut StringInterner,
    ) -> bool {
        // 1. Check built-in modules
        if let Some(module) = self.builtin_modules.get(name) {
            for &(fn_name, func) in &module.functions {
                let id = interner.intern(fn_name);
                natives.insert(id, CallableNative::Rust(func));
                globals.insert(id, Value::obj(id as u64));
            }
            for &(const_name, val) in &module.constants {
                let id = interner.intern(const_name);
                globals.insert(id, val);
            }
            return true;
        }

        // 2. Search for external shared object: lib<name>.so or <name>.so
        let filenames = [
            format!("lib{}.so", name),
            format!("{}.so", name),
            format!("lib{}.dylib", name),
            format!("{}.dylib", name),
        ];

        let search_paths = self.search_paths.clone();
        for search_dir in &search_paths {
            for filename in &filenames {
                let candidate = search_dir.join(filename);
                if candidate.exists() {
                    if let Ok(loaded) =
                        self.load_external_so(&candidate, natives, globals, interner)
                    {
                        if loaded {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Dynamically loads an external shared object file via C-ABI initialization.
    fn load_external_so(
        &mut self,
        path: &Path,
        natives: &mut HashMap<u32, CallableNative>,
        globals: &mut HashMap<u32, Value>,
        interner: &mut StringInterner,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        unsafe {
            let lib = libloading::Library::new(path)?;
            let init_fn: libloading::Symbol<unsafe extern "C" fn(*mut ElyApi) -> i32> =
                lib.get(b"ely_init_module\0")?;

            let mut load_ctx = LoadContext {
                natives,
                globals,
                interner,
            };

            let mut api = ElyApi {
                ctx: &mut load_ctx as *mut _ as *mut c_void,
                register_fn: c_register_fn,
                register_number: c_register_number,
            };

            let status = init_fn(&mut api);
            if status == 0 {
                self.loaded_libraries.push(lib);
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    /// Registers all built-in modules in one go into the VM tables.
    pub fn register_all_builtins(
        &self,
        natives: &mut HashMap<u32, CallableNative>,
        globals: &mut HashMap<u32, Value>,
        interner: &mut StringInterner,
    ) {
        for module in self.builtin_modules.values() {
            for &(fn_name, func) in &module.functions {
                let id = interner.intern(fn_name);
                natives.insert(id, CallableNative::Rust(func));
                globals.insert(id, Value::obj(id as u64));
            }
            for &(const_name, val) in &module.constants {
                let id = interner.intern(const_name);
                globals.insert(id, val);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_registry_builtin_math_and_io() {
        let mut registry = ModuleRegistry::new();
        let mut natives = HashMap::new();
        let mut globals = HashMap::new();
        let mut interner = StringInterner::new();

        assert!(registry.load("math", &mut natives, &mut globals, &mut interner));
        assert!(registry.load("io", &mut natives, &mut globals, &mut interner));
        assert!(registry.load("stdio", &mut natives, &mut globals, &mut interner));
        assert!(!registry.load(
            "nonexistent_module",
            &mut natives,
            &mut globals,
            &mut interner
        ));

        let sqrt_id = interner.intern("sqrt");
        assert!(natives.contains_key(&sqrt_id));
        assert!(globals.contains_key(&sqrt_id));

        let pi_id = interner.intern("PI");
        assert_eq!(
            globals.get(&pi_id).unwrap().as_number(),
            std::f64::consts::PI
        );
    }

    #[test]
    fn test_c_abi_registration_trampolines() {
        unsafe extern "C" fn foreign_add(argc: u8, args: *const Value) -> Value {
            let slice = std::slice::from_raw_parts(args, argc as usize);
            Value::number(slice[0].as_number() + slice[1].as_number())
        }

        let mut natives = HashMap::new();
        let mut globals = HashMap::new();
        let mut interner = StringInterner::new();

        let mut load_ctx = LoadContext {
            natives: &mut natives,
            globals: &mut globals,
            interner: &mut interner,
        };

        let api = ElyApi {
            ctx: &mut load_ctx as *mut _ as *mut c_void,
            register_fn: c_register_fn,
            register_number: c_register_number,
        };

        unsafe {
            let fn_name = std::ffi::CString::new("foreign_add").unwrap();
            (api.register_fn)(api.ctx, fn_name.as_ptr(), foreign_add);

            let const_name = std::ffi::CString::new("ANSWER").unwrap();
            (api.register_number)(api.ctx, const_name.as_ptr(), 42.0);
        }

        let fn_id = interner.intern("foreign_add");
        let handler = natives.get(&fn_id).unwrap();
        let args = [Value::number(10.0), Value::number(32.0)];
        let res = unsafe { handler.invoke(2, &args) };
        assert_eq!(res.as_number(), 42.0);

        let const_id = interner.intern("ANSWER");
        assert_eq!(globals.get(&const_id).unwrap().as_number(), 42.0);
    }
}
