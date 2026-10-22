use std::collections::HashMap;

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

    /// Interprets a compiled bytecode chunk by initializing the top-level call frame and dispatching.
    pub fn interpret(&mut self, chunk: &'a Chunk) -> InterpretResult {
        self.reset_stack();
        self.frames.push(CallFrame::new(chunk, 0));
        self.run()
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

    fn run(&mut self) -> InterpretResult {
        loop {
            let frame_idx = self.frames.len() - 1;
            let instruction = self.frames[frame_idx].read_byte();

            let opcode = match Opcode::try_from(instruction) {
                Ok(op) => op,
                Err(_) => return InterpretResult::RuntimeError,
            };

            match opcode {
                Opcode::Constant => {
                    let value = self.frames[frame_idx].read_constant();
                    self.push(value);
                }
                Opcode::True => self.push(Value::bool(true)),
                Opcode::False => self.push(Value::bool(false)),
                Opcode::Nop => self.push(Value::null()),
                Opcode::Pop => _ = self.pop(),
                Opcode::Return => {
                    self.pop();
                    return InterpretResult::Ok;
                }
                Opcode::Add => {
                    if !self.binary_op(|a, b| a + b) {
                        return InterpretResult::RuntimeError;
                    }
                }
                Opcode::Subtract => {
                    if !self.binary_op(|a, b| a - b) {
                        return InterpretResult::RuntimeError;
                    }
                }
                Opcode::Multiply => {
                    if !self.binary_op(|a, b| a * b) {
                        return InterpretResult::RuntimeError;
                    }
                }
                Opcode::Divide => {
                    if !self.binary_op(|a, b| a / b) {
                        return InterpretResult::RuntimeError;
                    }
                }
                Opcode::Negate => {
                    if !self.peek(0).is_number() {
                        return InterpretResult::RuntimeError;
                    }

                    let n = self.pop().as_number();
                    self.push(Value::number(-n));
                }
                Opcode::Not => {
                    let value = self.pop();
                    let is_falsy = value.is_null() || (value.is_bool() && !value.as_bool());
                    self.push(Value::bool(is_falsy));
                }
                Opcode::Equal => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(Value::bool(a.0 == b.0));
                }
                Opcode::Greater => {
                    if !self.peek(0).is_number() || !self.peek(1).is_number() {
                        return InterpretResult::RuntimeError;
                    }
                    let b = self.pop().as_number();
                    let a = self.pop().as_number();
                    self.push(Value::bool(a > b));
                }
                Opcode::Less => {
                    if !self.peek(0).is_number() || !self.peek(1).is_number() {
                        return InterpretResult::RuntimeError;
                    }
                    let b = self.pop().as_number();
                    let a = self.pop().as_number();
                    self.push(Value::bool(a < b));
                }
                Opcode::DefineGlobal => {
                    let name_id = self.frames[frame_idx].read_constant().as_obj() as u32;
                    let val = self.pop();
                    self.globals.insert(name_id, val);
                }
                Opcode::GetGlobal => {
                    let name_id = self.frames[frame_idx].read_constant().as_obj() as u32;
                    if let Some(val) = self.globals.get(&name_id) {
                        self.push(*val);
                    } else {
                        return InterpretResult::RuntimeError;
                    }
                }
                Opcode::SetGlobal => {
                    let name_id = self.frames[frame_idx].read_constant().as_obj() as u32;
                    if !self.globals.contains_key(&name_id) {
                        return InterpretResult::RuntimeError;
                    }
                    let value = self.peek(0);
                    self.globals.insert(name_id, value);
                }
                Opcode::GetLocal => {
                    let slot = self.frames[frame_idx].read_byte() as usize;
                    let value = self.stack[self.frames[frame_idx].slots + slot];
                    self.push(value);
                }
                Opcode::SetLocal => {
                    let slot = self.frames[frame_idx].read_byte() as usize;
                    let idx = self.frames[frame_idx].slots + slot;
                    let value = self.peek(0);
                    self.stack[idx] = value;
                }
                Opcode::Jump => {
                    let offset = self.frames[frame_idx].read_short();
                    self.frames[frame_idx].ip += offset as usize;
                }
                Opcode::JumpIfFalse => {
                    let is_falsy = {
                        let val = self.peek(0);
                        val.is_null() || (val.is_bool() && !val.as_bool())
                    };

                    let offset = self.frames[frame_idx].read_short();
                    if is_falsy {
                        self.frames[frame_idx].ip += offset as usize;
                    }
                }
                Opcode::Loop => {
                    let offset = self.frames[frame_idx].read_short();
                    self.frames[frame_idx].ip -= offset as usize;
                }
                _ => return InterpretResult::RuntimeError,
            }
        }
    }

    fn binary_op<F>(&mut self, op: F) -> bool
    where
        F: FnOnce(f64, f64) -> f64,
    {
        if !self.peek(0).is_number() || !self.peek(1).is_number() {
            return false;
        }

        let b = self.pop().as_number();
        let a = self.pop().as_number();
        self.push(Value::number(op(a, b)));
        true
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

    #[test]
    fn test_vm_interpret_constants_and_literals() {
        let mut chunk = Chunk::new();
        // push 42.0
        let c = chunk.add_constant(Value::number(42.0));
        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c as u8, 1);
        // push true, false, null
        chunk.write_byte(Opcode::True as u8, 1);
        chunk.write_byte(Opcode::False as u8, 1);
        chunk.write_byte(Opcode::Nop as u8, 1);
        // pop null, false, true
        chunk.write_byte(Opcode::Pop as u8, 1);
        chunk.write_byte(Opcode::Pop as u8, 1);
        chunk.write_byte(Opcode::Pop as u8, 1);
        // return 42.0
        chunk.write_byte(Opcode::Return as u8, 1);

        let mut vm = VM::new();
        let result = vm.interpret(&chunk);

        assert_eq!(result, InterpretResult::Ok);
    }

    #[test]
    fn test_vm_interpret_arithmetic_and_negate() {
        let mut chunk = Chunk::new();
        // 5 + 3 * 2 = 11
        let c1 = chunk.add_constant(Value::number(5.0));
        let c2 = chunk.add_constant(Value::number(3.0));
        let c3 = chunk.add_constant(Value::number(2.0));

        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c1 as u8, 1);

        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c2 as u8, 1);

        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c3 as u8, 1);

        chunk.write_byte(Opcode::Multiply as u8, 1);
        chunk.write_byte(Opcode::Add as u8, 1);
        chunk.write_byte(Opcode::Negate as u8, 1);
        // stack top is -11.0, return it
        chunk.write_byte(Opcode::Return as u8, 1);

        let mut vm = VM::new();
        let result = vm.interpret(&chunk);

        assert_eq!(result, InterpretResult::Ok);
    }

    #[test]
    fn test_vm_interpret_comparisons() {
        let mut chunk = Chunk::new();
        // 5 < 10 -> true
        let c1 = chunk.add_constant(Value::number(5.0));
        let c2 = chunk.add_constant(Value::number(10.0));

        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c1 as u8, 1);

        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c2 as u8, 1);

        chunk.write_byte(Opcode::Less as u8, 1);
        // check with Equal against true
        chunk.write_byte(Opcode::True as u8, 1);
        chunk.write_byte(Opcode::Equal as u8, 1);
        chunk.write_byte(Opcode::Return as u8, 1);

        let mut vm = VM::new();
        let result = vm.interpret(&chunk);

        assert_eq!(result, InterpretResult::Ok);
    }

    #[test]
    fn test_vm_interpret_globals_and_locals() {
        // var x = 10;
        // {
        //   var y = 20;
        //   x = y + 5;
        // }
        let mut chunk = Chunk::new();
        let name_x = chunk.add_constant(Value::obj(100)); // id 100
        let c_10 = chunk.add_constant(Value::number(10.0));
        let c_20 = chunk.add_constant(Value::number(20.0));
        let c_5 = chunk.add_constant(Value::number(5.0));

        // push 10, define global x
        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c_10 as u8, 1);
        chunk.write_byte(Opcode::DefineGlobal as u8, 1);
        chunk.write_byte(name_x as u8, 1);

        // push 20 (local slot 0)
        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c_20 as u8, 1);

        // get local slot 0
        chunk.write_byte(Opcode::GetLocal as u8, 1);
        chunk.write_byte(0, 1);

        // push 5, add -> 25
        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c_5 as u8, 1);
        chunk.write_byte(Opcode::Add as u8, 1);

        // set global x
        chunk.write_byte(Opcode::SetGlobal as u8, 1);
        chunk.write_byte(name_x as u8, 1);
        chunk.write_byte(Opcode::Pop as u8, 1);

        // pop local slot 0
        chunk.write_byte(Opcode::Pop as u8, 1);

        // return
        chunk.write_byte(Opcode::Nop as u8, 1);
        chunk.write_byte(Opcode::Return as u8, 1);

        let mut vm = VM::new();
        let result = vm.interpret(&chunk);

        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.globals.get(&100).unwrap().as_number(), 25.0);
    }

    #[test]
    fn test_vm_interpret_loops_and_jumps() {
        // Compile a real while loop from source through ely_compiler!
        // var sum = 0;
        // var i = 0;
        // while (i < 5) {
        //     sum = sum + i;
        //     i = i + 1;
        // }
        use ely_core::interner::StringInterner;
        let mut interner = StringInterner::new();
        let source = r#"
            var sum = 0;
            var i = 0;
            while (i < 5) {
                sum = sum + i;
                i = i + 1;
            }
        "#;
        let chunk =
            ely_compiler::compiler::compile(source, &mut interner).expect("Compilation failed.");

        let mut vm = VM::new();
        let result = vm.interpret(&chunk);

        assert_eq!(result, InterpretResult::Ok);

        let sum_id = interner.intern("sum");
        let sum_val = vm.globals.get(&sum_id).expect("sum global not found.");
        // sum = 0 + 1 + 2 + 3 + 4 = 10
        assert_eq!(sum_val.as_number(), 10.0);
    }
}
