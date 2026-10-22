use std::rc::Rc;

use ely_core::{chunk::Chunk, value::Value};

/// Represents a dynamically allocated object tracked on the VM heap.
#[derive(Debug, Clone, PartialEq)]
pub enum HeapObject {
    String(String),
    Array(Vec<Value>),
    Function(ObjFunction),
}

/// Represents a compiled function object with its bytecode and arity.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjFunction {
    pub arity: usize,
    pub chunk: Rc<Chunk>,
    pub name: Option<String>,
}

impl ObjFunction {
    /// Creates a new function object.
    pub fn new(arity: usize, name: Option<String>) -> Self {
        Self {
            arity,
            chunk: Rc::new(Chunk::new()),
            name,
        }
    }
}
