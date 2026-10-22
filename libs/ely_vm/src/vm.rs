use std::collections::HashMap;

use ely_core::chunk::Chunk;
use ely_core::value::Value;

#[derive(Debug, PartialEq, Eq)]
pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

pub struct VM<'a> {
    pub stack: Vec<Value>,
    pub frames: Vec<CallFrame<'a>>,
    pub globals: HashMap<u32, Value>,
}

impl<'a> VM<'a> {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(256),
            frames: Vec::with_capacity(64),
            globals: HashMap::new(),
        }
    }

    pub fn reset_stack(&mut self) {
        self.stack.clear();
        self.frames.clear();
    }

    pub fn push(&mut self, value: Value) {
        self.stack.push(value)
    }

    pub fn pop(&mut self) -> Value {
        self.stack.pop().expect("Stack underflow.")
    }

    pub fn peek(&self, distance: usize) -> Value {
        self.stack[self.stack.len() - 1 - distance]
    }
}

impl Default for VM<'_> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CallFrame<'a> {
    pub chunk: &'a Chunk,
    pub ip: usize,
    pub slots: usize,
}

impl<'a> CallFrame<'a> {
    pub fn new(chunk: &'a Chunk, slots: usize) -> Self {
        Self {
            chunk,
            ip: 0,
            slots,
        }
    }

    pub fn read_byte(&mut self) -> u8 {
        let byte = self.chunk.code[self.ip];
        self.ip += 1;
        byte
    }

    pub fn read_short(&mut self) -> u16 {
        let high = self.chunk.code[self.ip];
        let low = self.chunk.code[self.ip + 1];
        let short = ((high as u16) << 8u16) | low as u16;
        self.ip += 2;

        short as u16
    }

    pub fn read_constant(&mut self) -> Value {
        let index = self.read_byte();
        self.chunk.constants[index as usize]
    }
}
