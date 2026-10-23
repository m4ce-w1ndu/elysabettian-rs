use std::collections::HashMap;
use std::rc::Rc;

use ely_core::{chunk::Chunk, value::Value};

/// Represents a dynamically allocated object tracked on the VM heap.
#[derive(Debug, Clone, PartialEq)]
pub enum HeapObject {
    String(String),
    Array(Vec<Value>),
    Function(ObjFunction),
    Closure(ObjClosure),
    Upvalue(ObjUpvalue),
    Class(ObjClass),
    Instance(ObjInstance),
    BoundMethod(ObjBoundMethod),
}

/// Represents a method bound to a receiver instance.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjBoundMethod {
    pub receiver: Value,
    pub method_id: usize,
}

impl ObjBoundMethod {
    pub fn new(receiver: Value, method_id: usize) -> Self {
        Self {
            receiver,
            method_id,
        }
    }
}

/// Represents a class definition with its methods table.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjClass {
    pub name: String,
    pub methods: HashMap<u32, usize>,
}

impl ObjClass {
    pub fn new(name: String) -> Self {
        Self {
            name,
            methods: HashMap::new(),
        }
    }
}

/// Represents an instantiated class object with dynamic fields.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjInstance {
    pub class_id: usize,
    pub fields: HashMap<u32, Value>,
}

impl ObjInstance {
    pub fn new(class_id: usize) -> Self {
        Self {
            class_id,
            fields: HashMap::new(),
        }
    }
}

/// Represents a compiled function object with its bytecode and arity.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjFunction {
    pub arity: usize,
    pub upvalue_count: usize,
    pub chunk: Rc<Chunk>,
    pub name: Option<String>,
    pub invocation_count: usize,
    pub jit_entry: Option<*const u8>,
}

impl ObjFunction {
    /// Creates a new function object.
    pub fn new(arity: usize, name: Option<String>) -> Self {
        Self {
            arity,
            upvalue_count: 0,
            chunk: Rc::new(Chunk::new()),
            name,
            invocation_count: 0,
            jit_entry: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjUpvalue {
    pub location: usize,
    pub closed: Option<Value>,
}

impl ObjUpvalue {
    pub fn new(location: usize) -> Self {
        Self {
            location,
            closed: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.closed.is_none()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjClosure {
    pub function_id: usize,
    pub upvalues: Vec<usize>,
}

impl ObjClosure {
    pub fn new(function_id: usize) -> Self {
        Self {
            function_id,
            upvalues: Vec::new(),
        }
    }
}
