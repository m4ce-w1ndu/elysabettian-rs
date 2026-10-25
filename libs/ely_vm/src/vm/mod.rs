pub mod dispatch;
pub mod jit_call;
pub mod upvalue;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::rc::Rc;

use crate::frame::CallFrame;
use crate::heap::HeapObject;
use ely_core::chunk::Chunk;
use ely_core::opcode::Opcode;
use ely_core::value::Value;

/// Represents the outcome of interpreting a bytecode chunk.
#[derive(Debug, PartialEq, Eq)]
pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

pub use ely_stdlib::{CallableNative, NativeFn};

pub const MAX_FRAMES: usize = 64;

/// Hot path invocation threshold
pub const JIT_HOT_THRESHOLD: usize = 50;

/// The virtual machine responsible for executing compiled bytecode chunks.
pub struct VM {
    pub stack: Vec<Value>,
    pub frames: Vec<CallFrame>,
    pub globals: HashMap<u32, Value>,
    pub natives: HashMap<u32, CallableNative>,
    pub heap: Vec<HeapObject>,
    pub open_upvalues: Vec<usize>,
    pub jit_engine: Option<ely_jit::engine::JitEngine>,
    pub registry: ely_stdlib::ModuleRegistry,
    pub interner: ely_core::interner::StringInterner,
}

impl VM {
    /// Creates a new virtual machine instance with preallocated stack and call frames.
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(256),
            frames: Vec::with_capacity(64),
            globals: HashMap::new(),
            natives: HashMap::new(),
            heap: Vec::new(),
            open_upvalues: Vec::new(),
            jit_engine: ely_jit::engine::JitEngine::new().ok(),
            registry: ely_stdlib::ModuleRegistry::new(),
            interner: ely_core::interner::StringInterner::new(),
        }
    }

    /// Interprets a compiled bytecode chunk by initializing the top-level call frame and dispatching.
    pub fn interpret(&mut self, chunk: &Chunk) -> InterpretResult {
        self.reset_stack();
        // Slot 0 is reserved for the script/caller function (stack convention)
        self.push(Value::null());

        let mut chunk = chunk.clone();
        self.load_chunk_objects(&mut chunk);

        self.frames.push(CallFrame::new(Rc::new(chunk), 0));
        self.run()
    }

    /// Recursively loads string constants and compiled functions onto the heap and links their handles.
    fn load_chunk_objects(&mut self, chunk: &mut Chunk) {
        let mut string_vals = Vec::with_capacity(chunk.string_constants.len());
        for s in &chunk.string_constants {
            let val = self.allocate_object(HeapObject::String(s.clone()));
            ely_jit::runtime::append_jit_strings(&[s.clone()]);
            string_vals.push(val);
        }

        let mut func_vals = Vec::with_capacity(chunk.function_constants.len());
        for f in &mut chunk.function_constants {
            self.load_chunk_objects(&mut f.chunk);
            let mut obj_func = crate::heap::ObjFunction::new(f.arity, f.name.clone());
            obj_func.chunk = Rc::new(f.chunk.clone());
            let val = self.allocate_object(HeapObject::Function(obj_func));
            func_vals.push(val);
        }

        // Patch constants in chunk.constants using chunk.code opcode context
        let mut ip = 0;
        while ip < chunk.code.len() {
            if let Ok(opcode) = Opcode::try_from(chunk.code[ip]) {
                ip += 1;
                match opcode {
                    Opcode::Constant => {
                        let const_idx = chunk.code[ip] as usize;
                        ip += 1;
                        if let Some(c) = chunk.constants.get_mut(const_idx) {
                            if c.is_obj() {
                                let orig_idx = c.as_obj() as usize;
                                if let Some(&real_val) = string_vals.get(orig_idx) {
                                    *c = real_val;
                                }
                            }
                        }
                    }
                    Opcode::Closure => {
                        let const_idx = chunk.code[ip] as usize;
                        ip += 1;
                        if let Some(c) = chunk.constants.get_mut(const_idx) {
                            if c.is_obj() {
                                let orig_idx = c.as_obj() as usize;
                                if let Some(&real_val) = func_vals.get(orig_idx) {
                                    *c = real_val;
                                }
                            }
                        }
                    }
                    Opcode::GetLocal
                    | Opcode::SetLocal
                    | Opcode::GetGlobal
                    | Opcode::DefineGlobal
                    | Opcode::SetGlobal
                    | Opcode::Call
                    | Opcode::Class
                    | Opcode::GetProperty
                    | Opcode::SetProperty
                    | Opcode::Method
                    | Opcode::Invoke
                    | Opcode::SuperInvoke
                    | Opcode::Import => {
                        ip += 1;
                    }
                    Opcode::Jump | Opcode::JumpIfFalse | Opcode::Loop => {
                        ip += 2;
                    }
                    _ => {}
                }
            } else {
                ip += 1;
            }
        }
    }

    /// Resets the value stack and call frame stack.
    pub fn reset_stack(&mut self) {
        self.stack.clear();
        self.frames.clear();
        self.open_upvalues.clear();
    }

    /// Pushes a value onto the virtual machine stack.
    pub fn push(&mut self, value: Value) {
        self.stack.push(value)
    }

    /// Pops a value from the top of the stack.
    pub fn pop(&mut self) -> Value {
        self.stack.pop().expect("Stack underflow.")
    }

    /// Peeks a value at the specified distance from the top of the stack.
    pub fn peek(&self, distance: usize) -> Value {
        self.stack[self.stack.len() - 1 - distance]
    }

    pub fn define_native(&mut self, name_id: u32, function: impl Into<CallableNative>) {
        self.natives.insert(name_id, function.into());
    }

    /// Registers all standard library functions and constants.
    pub fn register_stdlib(&mut self, interner: &mut ely_core::interner::StringInterner) {
        self.registry
            .register_all_builtins(&mut self.natives, &mut self.globals, interner);
    }

    /// Allocates an object on the heap and returns a NaN-boxed Value reference.
    pub fn allocate_object(&mut self, object: HeapObject) -> Value {
        let id = self.heap.len();
        self.heap.push(object);
        Value::obj(id as u64)
    }

    /// Retrieves an immutable reference to a heap object.
    pub fn get_object(&self, value: Value) -> Option<&HeapObject> {
        if !value.is_obj() {
            return None;
        }
        let id = value.as_obj() as usize;
        self.heap.get(id)
    }

    /// Retrieves a mutable reference to a heap object.
    pub fn get_object_mut(&mut self, value: Value) -> Option<&mut HeapObject> {
        if !value.is_obj() {
            return None;
        }
        let id = value.as_obj() as usize;
        self.heap.get_mut(id)
    }

    pub fn format_value(&self, value: Value) -> String {
        if value.is_number() {
            value.as_number().to_string()
        } else if value.is_null() {
            "null".to_string()
        } else if value.is_bool() {
            value.as_bool().to_string()
        } else if value.is_obj() {
            match self.get_object(value) {
                Some(HeapObject::String(s)) => s.clone(),
                Some(HeapObject::Array(elements)) => {
                    let items: Vec<String> =
                        elements.iter().map(|&v| self.format_value(v)).collect();
                    format!("[{}]", items.join(", "))
                }
                Some(HeapObject::Class(c)) => format!("<class {}>", c.name),
                Some(HeapObject::Instance(i)) => format!("<instance of class {}>", i.class_id),
                Some(HeapObject::Function(f)) => {
                    if let Some(ref name) = f.name {
                        format!("<fn {}>", name)
                    } else {
                        "<script>".to_string()
                    }
                }
                Some(HeapObject::Closure(_)) => "closure".to_string(),
                Some(HeapObject::BoundMethod(_)) => "<bound method>".to_string(),
                Some(HeapObject::Upvalue(_)) => "<upvalue>".to_string(),
                None => format!("<obj 0x{:x}>", value.as_obj()),
            }
        } else {
            format!("<unknown 0x{:x}>", value.0)
        }
    }
}
