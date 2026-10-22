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
}

/// Represents a compiled function object with its bytecode and arity.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjFunction {
    pub arity: usize,
    pub upvalue_count: usize,
    pub chunk: Rc<Chunk>,
    pub name: Option<String>,
}

impl ObjFunction {
    /// Creates a new function object.
    pub fn new(arity: usize, name: Option<String>) -> Self {
        Self {
            arity,
            upvalue_count: 0,
            chunk: Rc::new(Chunk::new()),
            name,
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
