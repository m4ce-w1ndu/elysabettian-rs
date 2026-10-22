use std::collections::HashMap;

use ely_core::chunk::Chunk;
use ely_core::value::Value;

/// Represents the outcome of interpreting a bytecode chunk.
#[derive(Debug, PartialEq, Eq)]
pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

/// The virtual machine responsible for executing compiled bytecode chunks.
pub struct VM<'a> {
    pub stack: Vec<Value>,
    pub frames: Vec<CallFrame<'a>>,
    pub globals: HashMap<u32, Value>,
}

impl<'a> VM<'a> {
    /// Creates a new virtual machine instance with preallocated stack and call frames.
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(256),
            frames: Vec::with_capacity(64),
            globals: HashMap::new(),
        }
    }

    /// Resets the value stack and call frame stack.
    pub fn reset_stack(&mut self) {
        self.stack.clear();
        self.frames.clear();
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
}

impl Default for VM<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// A call frame representing an active function or script invocation.
pub struct CallFrame<'a> {
    pub chunk: &'a Chunk,
    pub ip: usize,
    pub slots: usize,
}

impl<'a> CallFrame<'a> {
    /// Creates a new call frame starting at offset 0 of the given chunk.
    pub fn new(chunk: &'a Chunk, slots: usize) -> Self {
        Self {
            chunk,
            ip: 0,
            slots,
        }
    }

    /// Reads the next byte from the chunk bytecode and advances the instruction pointer.
    pub fn read_byte(&mut self) -> u8 {
        let byte = self.chunk.code[self.ip];
        self.ip += 1;
        byte
    }

    /// Reads the next 16-bit big-endian unsigned integer and advances by two bytes.
    pub fn read_short(&mut self) -> u16 {
        let high = self.chunk.code[self.ip];
        let low = self.chunk.code[self.ip + 1];
        let short = ((high as u16) << 8u16) | low as u16;
        self.ip += 2;

        short
    }

    /// Reads the next byte as a constant index and retrieves the corresponding value.
    pub fn read_constant(&mut self) -> Value {
        let index = self.read_byte();
        self.chunk.constants[index as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_operations() {
        let mut vm = VM::new();
        vm.push(Value::number(10.0));
        vm.push(Value::number(20.0));

        assert_eq!(vm.peek(0).as_number(), 20.0);
        assert_eq!(vm.peek(1).as_number(), 10.0);

        assert_eq!(vm.pop().as_number(), 20.0);
        assert_eq!(vm.pop().as_number(), 10.0);
    }

    #[test]
    fn test_call_frame_readers() {
        let mut chunk = Chunk::new();
        chunk.write_byte(0x12, 1);
        chunk.write_byte(0x34, 1);
        let const_idx = chunk.add_constant(Value::number(99.0));
        chunk.write_byte(const_idx as u8, 1);

        let mut frame = CallFrame::new(&chunk, 0);

        assert_eq!(frame.read_short(), 0x1234);
        assert_eq!(frame.read_constant().as_number(), 99.0);
    }
}
