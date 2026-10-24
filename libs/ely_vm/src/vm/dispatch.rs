use crate::frame::CallFrame;
use crate::heap::HeapObject;
use crate::vm::{InterpretResult, MAX_FRAMES, VM};
use ely_core::opcode::Opcode;
use ely_core::value::Value;

impl VM {
    pub(crate) fn run(&mut self) -> InterpretResult {
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
                            let result = unsafe {
                                native_fn.invoke(arg_count as u8, &self.stack[args_start..])
                            };
                            // Remove args and callee
                            self.stack.truncate(args_start - 1);
                            self.push(result);
                        } else {
                            let func_id = callee.as_obj() as usize;
                            let is_func =
                                matches!(self.heap.get(func_id), Some(HeapObject::Function(_)));
                            if is_func {
                                match self.try_call_jit(func_id, arg_count) {
                                    Ok(true) => {
                                        // Executed natively via JIT
                                    }
                                    Ok(false) => {
                                        let func = match self.heap.get(func_id) {
                                            Some(HeapObject::Function(f)) => f,
                                            _ => return InterpretResult::RuntimeError,
                                        };

                                        if self.frames.len() == MAX_FRAMES {
                                            return InterpretResult::RuntimeError;
                                        }

                                        let slots = self.stack.len() - arg_count - 1;
                                        let chunk = func.chunk.clone();
                                        self.frames.push(CallFrame::new(chunk, slots));
                                    }
                                    Err(err) => return err,
                                }
                            } else {
                                match self.get_object(callee) {
                                    Some(HeapObject::Function(_)) => unreachable!(),
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
                                        self.frames.push(CallFrame::with_closure(
                                            chunk, slots, closure_id,
                                        ));
                                    }
                                    Some(HeapObject::Class(_)) => {
                                        let class_id = callee.as_obj() as usize;
                                        let instance_val =
                                            self.allocate_object(HeapObject::Instance(
                                                crate::heap::ObjInstance::new(class_id),
                                            ));
                                        let callee_slot = self.stack.len() - arg_count - 1;
                                        self.stack[callee_slot] = instance_val;
                                    }
                                    Some(HeapObject::BoundMethod(bound)) => {
                                        let receiver = bound.receiver;
                                        let method_id = bound.method_id;
                                        let callee_slot = self.stack.len() - arg_count - 1;
                                        self.stack[callee_slot] = receiver;

                                        let func = match self.heap.get(method_id) {
                                            Some(HeapObject::Function(f)) => f,
                                            Some(HeapObject::Closure(c)) => {
                                                match self.heap.get(c.function_id) {
                                                    Some(HeapObject::Function(f)) => f,
                                                    _ => return InterpretResult::RuntimeError,
                                                }
                                            }
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
                                        self.frames.push(CallFrame::new(chunk, slots));
                                    }
                                    _ => return InterpretResult::RuntimeError,
                                }
                            }
                        }
                    } else {
                        return InterpretResult::RuntimeError;
                    }
                }
                Opcode::Print => {
                    let value = self.pop();
                    println!("{}", self.format_value(value));
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
                            let bound_obj =
                                crate::heap::ObjBoundMethod::new(instance_val, method_id);
                            let bound_val =
                                self.allocate_object(HeapObject::BoundMethod(bound_obj));
                            self.push(bound_val);
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
                Opcode::Inherit => {
                    let subclass_val = self.peek(0);
                    let superclass_val = self.peek(1);

                    let super_methods = match self.get_object(superclass_val) {
                        Some(HeapObject::Class(c)) => c.methods.clone(),
                        _ => return InterpretResult::RuntimeError,
                    };

                    match self.get_object_mut(subclass_val) {
                        Some(HeapObject::Class(sub)) => {
                            for (name, method) in super_methods {
                                sub.methods.entry(name).or_insert(method);
                            }
                        }
                        _ => return InterpretResult::RuntimeError,
                    }

                    self.pop(); // Pop subclass
                }
                Opcode::GetSuper => {
                    let name_const = self.frames[frame_idx].read_constant();
                    let name_id = name_const.as_obj() as u32;

                    let superclass_val = self.pop();
                    let instance_val = self.peek(0);

                    let method_id = match self.get_object(superclass_val) {
                        Some(HeapObject::Class(c)) => match c.methods.get(&name_id) {
                            Some(&m) => m,
                            None => return InterpretResult::RuntimeError,
                        },
                        _ => return InterpretResult::RuntimeError,
                    };

                    let bound_obj = crate::heap::ObjBoundMethod::new(instance_val, method_id);
                    let bound_val = self.allocate_object(HeapObject::BoundMethod(bound_obj));
                    self.pop(); // Pop instance
                    self.push(bound_val);
                }
                Opcode::Invoke => {
                    let name_const = self.frames[frame_idx].read_constant();
                    let name_id = name_const.as_obj() as u32;
                    let arg_count = self.frames[frame_idx].read_byte() as usize;

                    let receiver_val = self.peek(arg_count);
                    let (class_id, field_val) = match self.get_object(receiver_val) {
                        Some(HeapObject::Instance(inst)) => {
                            let field = inst.fields.get(&name_id).copied();
                            (inst.class_id, field)
                        }
                        _ => return InterpretResult::RuntimeError,
                    };

                    if let Some(val) = field_val {
                        let receiver_slot = self.stack.len() - arg_count - 1;
                        self.stack[receiver_slot] = val;
                        // Call field value
                        if let Some(&native_fn) = self.natives.get(&(val.as_obj() as u32)) {
                            let args_start = self.stack.len() - arg_count;
                            let result = unsafe {
                                native_fn.invoke(arg_count as u8, &self.stack[args_start..])
                            };
                            self.stack.truncate(args_start - 1);
                            self.push(result);
                        } else {
                            match self.get_object(val) {
                                Some(HeapObject::Function(func)) => {
                                    if arg_count != func.arity || self.frames.len() == MAX_FRAMES {
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
                                    if arg_count != func.arity || self.frames.len() == MAX_FRAMES {
                                        return InterpretResult::RuntimeError;
                                    }
                                    let slots = self.stack.len() - arg_count - 1;
                                    let chunk = func.chunk.clone();
                                    let closure_id = val.as_obj() as usize;
                                    self.frames
                                        .push(CallFrame::with_closure(chunk, slots, closure_id));
                                }
                                _ => return InterpretResult::RuntimeError,
                            }
                        }
                    } else {
                        // Invoke method from class
                        let method_id = match self.heap.get(class_id) {
                            Some(HeapObject::Class(c)) => match c.methods.get(&name_id) {
                                Some(&m) => m,
                                None => return InterpretResult::RuntimeError,
                            },
                            _ => return InterpretResult::RuntimeError,
                        };

                        let (func_chunk, arity, closure_id) = match self.heap.get(method_id) {
                            Some(HeapObject::Function(f)) => (f.chunk.clone(), f.arity, None),
                            Some(HeapObject::Closure(c)) => {
                                let f = match self.heap.get(c.function_id) {
                                    Some(HeapObject::Function(func)) => func,
                                    _ => return InterpretResult::RuntimeError,
                                };
                                (f.chunk.clone(), f.arity, Some(method_id))
                            }
                            _ => return InterpretResult::RuntimeError,
                        };

                        if arg_count != arity || self.frames.len() == MAX_FRAMES {
                            return InterpretResult::RuntimeError;
                        }

                        let slots = self.stack.len() - arg_count - 1;
                        if let Some(cid) = closure_id {
                            self.frames
                                .push(CallFrame::with_closure(func_chunk, slots, cid));
                        } else {
                            self.frames.push(CallFrame::new(func_chunk, slots));
                        }
                    }
                }
                Opcode::SuperInvoke => {
                    let name_const = self.frames[frame_idx].read_constant();
                    let name_id = name_const.as_obj() as u32;
                    let arg_count = self.frames[frame_idx].read_byte() as usize;

                    let superclass_val = self.pop();
                    let method_id = match self.get_object(superclass_val) {
                        Some(HeapObject::Class(c)) => match c.methods.get(&name_id) {
                            Some(&m) => m,
                            None => return InterpretResult::RuntimeError,
                        },
                        _ => return InterpretResult::RuntimeError,
                    };

                    let (func_chunk, arity, closure_id) = match self.heap.get(method_id) {
                        Some(HeapObject::Function(f)) => (f.chunk.clone(), f.arity, None),
                        Some(HeapObject::Closure(c)) => {
                            let f = match self.heap.get(c.function_id) {
                                Some(HeapObject::Function(func)) => func,
                                _ => return InterpretResult::RuntimeError,
                            };
                            (f.chunk.clone(), f.arity, Some(method_id))
                        }
                        _ => return InterpretResult::RuntimeError,
                    };

                    if arg_count != arity || self.frames.len() == MAX_FRAMES {
                        return InterpretResult::RuntimeError;
                    }

                    let slots = self.stack.len() - arg_count - 1;
                    if let Some(cid) = closure_id {
                        self.frames
                            .push(CallFrame::with_closure(func_chunk, slots, cid));
                    } else {
                        self.frames.push(CallFrame::new(func_chunk, slots));
                    }
                }
                Opcode::Import => {
                    let const_idx = self.frames[frame_idx].read_byte() as usize;
                    let module_name =
                        match self.frames[frame_idx].chunk.string_constants.get(const_idx) {
                            Some(s) => s.clone(),
                            None => return InterpretResult::RuntimeError,
                        };
                    if !self.registry.load(
                        &module_name,
                        &mut self.natives,
                        &mut self.globals,
                        &mut self.interner,
                    ) {
                        eprintln!("Runtime Error: Failed to import module '{}'.", module_name);
                        return InterpretResult::RuntimeError;
                    }
                }
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
