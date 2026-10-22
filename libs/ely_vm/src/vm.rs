use std::collections::HashMap;
use std::rc::Rc;

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

pub type NativeFn = fn(arg_count: u8, args: &[Value]) -> Value;

pub const MAX_FRAMES: usize = 64;

/// The virtual machine responsible for executing compiled bytecode chunks.
pub struct VM {
    pub stack: Vec<Value>,
    pub frames: Vec<CallFrame>,
    pub globals: HashMap<u32, Value>,
    pub natives: HashMap<u32, NativeFn>,
    pub heap: Vec<HeapObject>,
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
        }
    }

    /// Interprets a compiled bytecode chunk by initializing the top-level call frame and dispatching.
    pub fn interpret(&mut self, chunk: &Chunk) -> InterpretResult {
        self.reset_stack();
        self.frames.push(CallFrame::new(Rc::new(chunk.clone()), 0));
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

    pub fn define_native(&mut self, name_id: u32, function: NativeFn) {
        self.natives.insert(name_id, function);
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
                    let result = self.pop();
                    let closing_frame = self.frames.pop().unwrap();

                    if self.frames.is_empty() {
                        return InterpretResult::Ok;
                    }

                    self.stack.truncate(closing_frame.slots);
                    self.push(result);
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
                Opcode::BwAnd => {
                    if !self.binary_op(|a, b| (a as i64 & b as i64) as f64) {
                        return InterpretResult::RuntimeError;
                    }
                }
                Opcode::BwOr => {
                    if !self.binary_op(|a, b| (a as i64 | b as i64) as f64) {
                        return InterpretResult::RuntimeError;
                    }
                }
                Opcode::BwXor => {
                    if !self.binary_op(|a, b| (a as i64 ^ b as i64) as f64) {
                        return InterpretResult::RuntimeError;
                    }
                }
                Opcode::ShiftLeft => {
                    if !self.binary_op(|a, b| ((a as i64) << (b as i64)) as f64) {
                        return InterpretResult::RuntimeError;
                    }
                }
                Opcode::ShiftRight => {
                    if !self.binary_op(|a, b| ((a as i64) >> (b as i64)) as f64) {
                        return InterpretResult::RuntimeError;
                    }
                }
                Opcode::BwNot => {
                    let a = self.pop();

                    if !a.is_number() {
                        return InterpretResult::RuntimeError;
                    }

                    let a = a.as_number() as i64;
                    self.push(Value::number((!a) as f64));
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
                Opcode::Call => {
                    let arg_count = self.frames[frame_idx].read_byte() as usize;
                    let callee = self.peek(arg_count);

                    if callee.is_obj() {
                        let name_id = callee.as_obj() as u32;
                        if let Some(&native_fn) = self.natives.get(&name_id) {
                            let args_start = self.stack.len() - arg_count;
                            let result = native_fn(arg_count as u8, &self.stack[args_start..]);
                            // Remove args and callee
                            self.stack.truncate(args_start - 1);
                            self.push(result);
                        } else {
                            match self.get_object(callee) {
                                Some(HeapObject::Function(func)) => {
                                    if arg_count != func.arity {
                                        return InterpretResult::RuntimeError;
                                    }

                                    if self.frames.len() == MAX_FRAMES {
                                        return InterpretResult::RuntimeError;
                                    }

                                    let slots = self.stack.len() - arg_count - 1;
                                    let chunk = func.chunk.clone();
                                    self.frames.push(CallFrame::new(chunk, slots));
                                }
                                _ => return InterpretResult::RuntimeError,
                            }
                        }
                    } else {
                        return InterpretResult::RuntimeError;
                    }
                }
                Opcode::Print => {
                    let value = self.pop();
                    println!("{}", value);
                }
                Opcode::ArrayBuild => {
                    let count = self.frames[frame_idx].read_byte() as usize;
                    let start = self.stack.len() - count;
                    let elements = self.stack.split_off(start);
                    let obj = self.allocate_object(HeapObject::Array(elements));
                    self.push(obj);
                }
                Opcode::ArrayIndex => {
                    let index_value = self.pop();
                    let array_value = self.pop();

                    if !index_value.is_number() {
                        return InterpretResult::RuntimeError;
                    }

                    let idx = index_value.as_number() as usize;
                    match self.get_object(array_value) {
                        Some(HeapObject::Array(elements)) => {
                            if idx >= elements.len() {
                                return InterpretResult::RuntimeError;
                            }

                            let item = elements[idx];
                            self.push(item);
                        }
                        _ => return InterpretResult::RuntimeError,
                    }
                }
                Opcode::ArrayStore => {
                    let val = self.pop();
                    let index_value = self.pop();
                    let array_value = self.pop();

                    if !index_value.is_number() {
                        return InterpretResult::RuntimeError;
                    }

                    let idx = index_value.as_number() as usize;
                    match self.get_object_mut(array_value) {
                        Some(HeapObject::Array(elements)) => {
                            if idx >= elements.len() {
                                return InterpretResult::RuntimeError;
                            }

                            elements[idx] = val;
                            self.push(val);
                        }
                        _ => return InterpretResult::RuntimeError,
                    }
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

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

/// A call frame representing an active function or script invocation.
pub struct CallFrame {
    pub chunk: Rc<Chunk>,
    pub ip: usize,
    pub slots: usize,
}

impl CallFrame {
    /// Creates a new call frame starting at offset 0 of the given chunk.
    pub fn new(chunk: Rc<Chunk>, slots: usize) -> Self {
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

        let mut frame = CallFrame::new(Rc::new(chunk), 0);

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

    #[test]
    fn test_vm_interpret_native_function_call() {
        use ely_core::interner::StringInterner;
        let mut interner = StringInterner::new();

        // Native function that adds two numbers
        fn native_add(_argc: u8, args: &[Value]) -> Value {
            let a = args[0].as_number();
            let b = args[1].as_number();
            Value::number(a + b)
        }

        let mut vm = VM::new();
        let add_id = interner.intern("add");
        // Register native function and expose as global variable
        vm.define_native(add_id, native_add);
        vm.globals.insert(add_id, Value::obj(add_id as u64));

        let source = "var result = add(10, 32);";
        let chunk =
            ely_compiler::compiler::compile(source, &mut interner).expect("Compilation failed.");

        let status = vm.interpret(&chunk);
        assert_eq!(status, InterpretResult::Ok);

        let result_id = interner.intern("result");
        let result_val = vm
            .globals
            .get(&result_id)
            .expect("result global not found.");
        assert_eq!(result_val.as_number(), 42.0);
    }

    #[test]
    fn test_vm_interpret_bitwise_operators() {
        let mut chunk = Chunk::new();
        // (12 & 10) | (1 << 2) ^ (16 >> 2)
        // 12 & 10 = 8
        // 1 << 2 = 4
        // 8 | 4 = 12
        // 16 >> 2 = 4
        // 12 ^ 4 = 8
        let c12 = chunk.add_constant(Value::number(12.0));
        let c10 = chunk.add_constant(Value::number(10.0));
        let c1 = chunk.add_constant(Value::number(1.0));
        let c2 = chunk.add_constant(Value::number(2.0));
        let c16 = chunk.add_constant(Value::number(16.0));

        // 12 & 10
        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c12 as u8, 1);
        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c10 as u8, 1);
        chunk.write_byte(Opcode::BwAnd as u8, 1);

        // 1 << 2
        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c1 as u8, 1);
        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c2 as u8, 1);
        chunk.write_byte(Opcode::ShiftLeft as u8, 1);

        // |
        chunk.write_byte(Opcode::BwOr as u8, 1);

        // 16 >> 2
        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c16 as u8, 1);
        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c2 as u8, 1);
        chunk.write_byte(Opcode::ShiftRight as u8, 1);

        // ^
        chunk.write_byte(Opcode::BwXor as u8, 1);

        // ~8 = -9
        chunk.write_byte(Opcode::BwNot as u8, 1);
        chunk.write_byte(Opcode::Return as u8, 1);

        let mut vm = VM::new();
        let status = vm.interpret(&chunk);
        assert_eq!(status, InterpretResult::Ok);
    }

    #[test]
    fn test_vm_interpret_arrays_build_index_store() {
        // [10, 20, 30] in local slot 0
        let mut chunk = Chunk::new();
        let c10 = chunk.add_constant(Value::number(10.0));
        let c20 = chunk.add_constant(Value::number(20.0));
        let c30 = chunk.add_constant(Value::number(30.0));
        let c1 = chunk.add_constant(Value::number(1.0));
        let c99 = chunk.add_constant(Value::number(99.0));

        // push 10, 20, 30
        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c10 as u8, 1);
        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c20 as u8, 1);
        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c30 as u8, 1);
        // build array (now on top of stack at local slot 0)
        chunk.write_byte(Opcode::ArrayBuild as u8, 1);
        chunk.write_byte(3, 1);

        // arr[1] = 99: stack needs: arr, 1, 99
        chunk.write_byte(Opcode::GetLocal as u8, 1);
        chunk.write_byte(0, 1);
        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c1 as u8, 1);
        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c99 as u8, 1);
        chunk.write_byte(Opcode::ArrayStore as u8, 1);
        chunk.write_byte(Opcode::Pop as u8, 1);

        // retrieve arr[1]: stack needs: arr, 1
        chunk.write_byte(Opcode::GetLocal as u8, 1);
        chunk.write_byte(0, 1);
        chunk.write_byte(Opcode::Constant as u8, 1);
        chunk.write_byte(c1 as u8, 1);
        chunk.write_byte(Opcode::ArrayIndex as u8, 1);
        chunk.write_byte(Opcode::Return as u8, 1);

        let mut vm = VM::new();
        let status = vm.interpret(&chunk);
        assert_eq!(status, InterpretResult::Ok);
    }

    #[test]
    fn test_vm_interpret_user_function_call() {
        // Define a function `double(x)` that returns `x * 2`
        // In the function's bytecode chunk:
        // slot 0: callee (double)
        // slot 1: parameter x
        let mut func_chunk = Chunk::new();
        func_chunk.write_byte(Opcode::GetLocal as u8, 1);
        func_chunk.write_byte(1, 1); // parameter x
        let c2 = func_chunk.add_constant(Value::number(2.0));
        func_chunk.write_byte(Opcode::Constant as u8, 1);
        func_chunk.write_byte(c2 as u8, 1);
        func_chunk.write_byte(Opcode::Multiply as u8, 1);
        func_chunk.write_byte(Opcode::Return as u8, 1);

        let mut func = crate::heap::ObjFunction::new(1, Some("double".to_string()));
        func.chunk = Rc::new(func_chunk);

        let mut vm = VM::new();
        let func_val = vm.allocate_object(HeapObject::Function(func));

        // In main script:
        // push func, push 21, Call 1, Return
        let mut main_chunk = Chunk::new();
        let c_func = main_chunk.add_constant(func_val);
        let c_21 = main_chunk.add_constant(Value::number(21.0));

        main_chunk.write_byte(Opcode::Constant as u8, 1);
        main_chunk.write_byte(c_func as u8, 1);
        main_chunk.write_byte(Opcode::Constant as u8, 1);
        main_chunk.write_byte(c_21 as u8, 1);
        main_chunk.write_byte(Opcode::Call as u8, 1);
        main_chunk.write_byte(1, 1); // 1 argument
        main_chunk.write_byte(Opcode::Return as u8, 1);

        let status = vm.interpret(&main_chunk);
        assert_eq!(status, InterpretResult::Ok);
    }
}
