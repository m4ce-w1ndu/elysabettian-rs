use std::collections::HashMap;
use std::rc::Rc;

use crate::heap::{HeapObject, ObjUpvalue};
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
    pub open_upvalues: Vec<usize>,
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
                    self.close_upvalue(closing_frame.slots);

                    self.stack.truncate(closing_frame.slots);
                    self.push(result);

                    if self.frames.is_empty() {
                        return InterpretResult::Ok;
                    }
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
                Opcode::GetUpvalue => {
                    let slot = self.frames[frame_idx].read_byte() as usize;
                    let closure_id = match self.frames[frame_idx].closure_id {
                        Some(id) => id,
                        None => return InterpretResult::RuntimeError,
                    };

                    if let Some(val) = self.get_upvalue_value(closure_id, slot) {
                        self.push(val);
                    } else {
                        return InterpretResult::RuntimeError;
                    }
                }
                Opcode::SetUpvalue => {
                    let slot = self.frames[frame_idx].read_byte() as usize;
                    let closure_id = match self.frames[frame_idx].closure_id {
                        Some(id) => id,
                        None => return InterpretResult::RuntimeError,
                    };
                    let val = self.peek(0);

                    if !self.set_upvalue_value(closure_id, slot, val) {
                        return InterpretResult::RuntimeError;
                    }
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
                                Some(HeapObject::Closure(closure)) => {
                                    let func = match self.heap.get(closure.function_id) {
                                        Some(HeapObject::Function(f)) => f,
                                        _ => return InterpretResult::RuntimeError,
                                    };

                                    if arg_count != func.arity {
                                        return InterpretResult::RuntimeError;
                                    }

                                    if self.frames.len() == MAX_FRAMES {
                                        return InterpretResult::RuntimeError;
                                    }

                                    let slots = self.stack.len() - arg_count - 1;
                                    let chunk = func.chunk.clone();
                                    let closure_id = callee.as_obj() as usize;
                                    self.frames
                                        .push(CallFrame::with_closure(chunk, slots, closure_id));
                                }
                                Some(HeapObject::Class(_)) => {
                                    let class_id = callee.as_obj() as usize;
                                    let instance_val = self.allocate_object(HeapObject::Instance(
                                        crate::heap::ObjInstance::new(class_id),
                                    ));
                                    let callee_slot = self.stack.len() - arg_count - 1;
                                    self.stack[callee_slot] = instance_val;
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
                Opcode::CloseUpvalue => {
                    self.close_upvalue(self.stack.len() - 1);
                    self.pop();
                }
                Opcode::Closure => {
                    let const_val = self.frames[frame_idx].read_constant();
                    let func_id = const_val.as_obj() as usize;

                    let upvalue_count = match self.heap.get(func_id) {
                        Some(HeapObject::Function(f)) => f.upvalue_count,
                        _ => return InterpretResult::RuntimeError,
                    };

                    let current_slots = self.frames[frame_idx].slots;
                    let current_closure_id = self.frames[frame_idx].closure_id;

                    let mut closure = crate::heap::ObjClosure::new(func_id);

                    for _ in 0..upvalue_count {
                        let is_local = self.frames[frame_idx].read_byte();
                        let index = self.frames[frame_idx].read_byte() as usize;

                        let upvalue_id = if is_local == 1 {
                            self.capture_upvalue(current_slots + index)
                        } else {
                            // Inherits from enclosing closure
                            let enclosing_id = current_closure_id.expect("No enclosing closure");
                            match self.heap.get(enclosing_id) {
                                Some(HeapObject::Closure(c)) => c.upvalues[index],
                                _ => return InterpretResult::RuntimeError,
                            }
                        };

                        closure.upvalues.push(upvalue_id);
                    }

                    let closure_val = self.allocate_object(HeapObject::Closure(closure));
                    self.push(closure_val);
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
                Opcode::Class => {
                    let name_val = self.frames[frame_idx].read_constant();
                    let name = format!("Class_{}", name_val.as_obj());
                    let class_obj = crate::heap::ObjClass::new(name);
                    let val = self.allocate_object(HeapObject::Class(class_obj));
                    self.push(val);
                }
                Opcode::Method => {
                    let name_const = self.frames[frame_idx].read_constant();
                    let name_id = name_const.as_obj() as u32;

                    let method_val = self.pop();
                    let class_val = self.peek(0);

                    let method_id = method_val.as_obj() as usize;
                    if let Some(HeapObject::Class(class_obj)) = self.get_object_mut(class_val) {
                        class_obj.methods.insert(name_id, method_id);
                    } else {
                        return InterpretResult::RuntimeError;
                    }
                }
                Opcode::GetProperty => {
                    let name_const = self.frames[frame_idx].read_constant();
                    let name_id = name_const.as_obj() as u32;

                    let instance_val = self.pop();
                    if let Some(HeapObject::Instance(instance)) = self.get_object(instance_val) {
                        if let Some(&field_val) = instance.fields.get(&name_id) {
                            self.push(field_val);
                        } else {
                            // Method binding fallback
                            let class_id = instance.class_id;
                            let method_id = match self.heap.get(class_id) {
                                Some(HeapObject::Class(c)) => match c.methods.get(&name_id) {
                                    Some(&m) => m,
                                    None => return InterpretResult::RuntimeError,
                                },
                                _ => return InterpretResult::RuntimeError,
                            };
                            self.push(Value::obj(method_id as u64));
                        }
                    } else {
                        return InterpretResult::RuntimeError;
                    }
                }
                Opcode::SetProperty => {
                    let name_const = self.frames[frame_idx].read_constant();
                    let name_id = name_const.as_obj() as u32;

                    let value = self.pop();
                    let instance_val = self.pop();

                    if let Some(HeapObject::Instance(instance)) = self.get_object_mut(instance_val)
                    {
                        instance.fields.insert(name_id, value);
                        self.push(value);
                    } else {
                        return InterpretResult::RuntimeError;
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

    fn capture_upvalue(&mut self, location: usize) -> usize {
        let existing = self.open_upvalues.iter().copied().find(|&id| {
            matches!(self.heap.get(id), Some(HeapObject::Upvalue(u)) if u.is_open() && u.location == location)
        });

        if let Some(id) = existing {
            return id;
        }

        let id = self.heap.len();
        self.heap
            .push(HeapObject::Upvalue(ObjUpvalue::new(location)));
        self.open_upvalues.push(id);

        id
    }

    fn close_upvalue(&mut self, last_slot: usize) {
        let stack = &self.stack;
        self.open_upvalues.retain(|&id| {
            if let Some(HeapObject::Upvalue(upvalue)) = self.heap.get_mut(id) {
                if upvalue.location >= last_slot {
                    upvalue.closed = Some(stack[upvalue.location]);
                    return false;
                }
            }

            true
        })
    }

    fn get_upvalue_value(&self, closure_id: usize, slot: usize) -> Option<Value> {
        let upvalue_id = match self.heap.get(closure_id)? {
            HeapObject::Closure(closure) => *closure.upvalues.get(slot)?,
            _ => return None,
        };

        match self.heap.get(upvalue_id)? {
            HeapObject::Upvalue(upvalue) => {
                if upvalue.is_open() {
                    Some(self.stack[upvalue.location])
                } else {
                    upvalue.closed
                }
            }
            _ => None,
        }
    }

    fn set_upvalue_value(&mut self, closure_id: usize, slot: usize, value: Value) -> bool {
        let upvalue_id = match self.heap.get(closure_id) {
            Some(HeapObject::Closure(closure)) => match closure.upvalues.get(slot) {
                Some(&id) => id,
                None => return false,
            },
            _ => return false,
        };

        let location = match self.heap.get(upvalue_id) {
            Some(HeapObject::Upvalue(upvalue)) => {
                if upvalue.is_open() {
                    Some(upvalue.location)
                } else {
                    None
                }
            }
            _ => return false,
        };

        if let Some(loc) = location {
            self.stack[loc] = value;
            true
        } else {
            match self.heap.get_mut(upvalue_id) {
                Some(HeapObject::Upvalue(upvalue)) => {
                    upvalue.closed = Some(value);
                    true
                }
                _ => false,
            }
        }
    }
}

/// Represents an active call frame in the call stack.
pub struct CallFrame {
    pub chunk: Rc<Chunk>,
    pub ip: usize,
    pub slots: usize,
    pub closure_id: Option<usize>,
}

impl CallFrame {
    pub fn new(chunk: Rc<Chunk>, slots: usize) -> Self {
        Self {
            chunk,
            ip: 0,
            slots,
            closure_id: None,
        }
    }

    pub fn with_closure(chunk: Rc<Chunk>, slots: usize, closure_id: usize) -> Self {
        Self {
            chunk,
            ip: 0,
            slots,
            closure_id: Some(closure_id),
        }
    }

    pub fn read_byte(&mut self) -> u8 {
        let byte = self.chunk.code[self.ip];
        self.ip += 1;
        byte
    }

    pub fn read_short(&mut self) -> u16 {
        self.ip += 2;
        ((self.chunk.code[self.ip - 2] as u16) << 8) | (self.chunk.code[self.ip - 1] as u16)
    }

    pub fn read_constant(&mut self) -> Value {
        let index = self.read_byte() as usize;
        self.chunk.constants[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emit_constant(chunk: &mut Chunk, val: Value, line: u32) {
        let idx = chunk.add_constant(val);
        chunk.write_opcode(Opcode::Constant, line);
        chunk.write_byte(idx as u8, line);
    }

    #[test]
    fn test_vm_interpret_simple_return() {
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        emit_constant(&mut chunk, Value::number(42.0), 1);
        chunk.write_opcode(Opcode::Return, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.stack.len(), 1);
        assert_eq!(vm.peek(0), Value::number(42.0));
    }

    #[test]
    fn test_vm_interpret_binary_arithmetic() {
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        emit_constant(&mut chunk, Value::number(10.0), 1);
        emit_constant(&mut chunk, Value::number(5.0), 1);
        chunk.write_opcode(Opcode::Add, 1);
        emit_constant(&mut chunk, Value::number(2.0), 1);
        chunk.write_opcode(Opcode::Multiply, 1);
        chunk.write_opcode(Opcode::Return, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.stack.len(), 1);
        assert_eq!(vm.peek(0), Value::number(30.0));
    }

    #[test]
    fn test_vm_interpret_bitwise_operations() {
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        emit_constant(&mut chunk, Value::number(12.0), 1);
        emit_constant(&mut chunk, Value::number(5.0), 1);
        chunk.write_opcode(Opcode::BwAnd, 1);
        emit_constant(&mut chunk, Value::number(3.0), 1);
        chunk.write_opcode(Opcode::BwOr, 1);
        chunk.write_opcode(Opcode::Return, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.stack.len(), 1);
        assert_eq!(vm.peek(0), Value::number(7.0));
    }

    #[test]
    fn test_vm_interpret_array_operations() {
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        emit_constant(&mut chunk, Value::number(10.0), 1);
        emit_constant(&mut chunk, Value::number(20.0), 1);
        emit_constant(&mut chunk, Value::number(30.0), 1);
        chunk.write_opcode(Opcode::ArrayBuild, 1);
        chunk.write_byte(3, 1);

        emit_constant(&mut chunk, Value::number(1.0), 1);
        chunk.write_opcode(Opcode::ArrayIndex, 1);

        chunk.write_opcode(Opcode::Return, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.stack.len(), 1);
        assert_eq!(vm.peek(0), Value::number(20.0));
    }

    #[test]
    fn test_vm_interpret_class_instance_and_properties() {
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        // 1. Declare class
        let class_name_const = chunk.add_constant(Value::obj(1));
        chunk.write_opcode(Opcode::Class, 1);
        chunk.write_byte(class_name_const as u8, 1);

        // 2. Instantiate class: Call with 0 args
        chunk.write_opcode(Opcode::Call, 1);
        chunk.write_byte(0, 1);

        // Store instance in local slot 0
        chunk.write_opcode(Opcode::SetLocal, 1);
        chunk.write_byte(0, 1);

        // Set property field 42 = 999.0: pushes instance, pushes value
        chunk.write_opcode(Opcode::GetLocal, 1);
        chunk.write_byte(0, 1);
        let field_name_const = chunk.add_constant(Value::obj(42));
        emit_constant(&mut chunk, Value::number(999.0), 1);
        chunk.write_opcode(Opcode::SetProperty, 1);
        chunk.write_byte(field_name_const as u8, 1);
        // Pop result of SetProperty
        chunk.write_opcode(Opcode::Pop, 1);

        // Get property field 42: push instance, GetProperty
        chunk.write_opcode(Opcode::GetLocal, 1);
        chunk.write_byte(0, 1);
        chunk.write_opcode(Opcode::GetProperty, 1);
        chunk.write_byte(field_name_const as u8, 1);

        chunk.write_opcode(Opcode::Return, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.peek(0), Value::number(999.0));
    }
}
