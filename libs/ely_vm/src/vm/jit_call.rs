use crate::heap::HeapObject;
use crate::vm::{InterpretResult, JIT_HOT_THRESHOLD, VM};
use ely_core::value::Value;
use std::mem;

impl VM {
    /// Attempts to execute a function via native JIT compilation if the hot-path threshold is reached.
    pub(crate) fn try_call_jit(
        &mut self,
        func_id: usize,
        arg_count: usize,
    ) -> Result<bool, InterpretResult> {
        let (arity, chunk, is_hot, existing_jit) = match self.heap.get_mut(func_id) {
            Some(HeapObject::Function(func)) => {
                if arg_count != func.arity {
                    return Err(InterpretResult::RuntimeError);
                }

                func.invocation_count += 1;
                let is_hot = func.invocation_count >= JIT_HOT_THRESHOLD && func.jit_entry.is_none();
                (func.arity, func.chunk.clone(), is_hot, func.jit_entry)
            }
            _ => return Err(InterpretResult::RuntimeError),
        };

        let mut compiled_jit = None;

        if is_hot {
            if let Some(ref mut engine) = self.jit_engine {
                let func_name = format!("jit_fn_{}", func_id);
                let mut compiler = ely_jit::compiler::JitCompiler::new(engine);
                if let Ok(code_ptr) = compiler.compile_function(&chunk, arity, &func_name) {
                    compiled_jit = Some(code_ptr);
                    if let Some(HeapObject::Function(func)) = self.heap.get_mut(func_id) {
                        func.jit_entry = Some(code_ptr);
                    }
                }
            }
        }

        if let Some(code_ptr) = existing_jit.or(compiled_jit) {
            let result_num = match arity {
                0 => {
                    let native_fn: extern "C" fn() -> f64 = unsafe { mem::transmute(code_ptr) };
                    native_fn()
                }
                1 => {
                    let native_fn: extern "C" fn(f64) -> f64 = unsafe { mem::transmute(code_ptr) };
                    let arg0 = self.stack[self.stack.len() - 1].as_number();
                    native_fn(arg0)
                }
                2 => {
                    let native_fn: extern "C" fn(f64, f64) -> f64 =
                        unsafe { mem::transmute(code_ptr) };
                    let arg0 = self.stack[self.stack.len() - 2].as_number();
                    let arg1 = self.stack[self.stack.len() - 1].as_number();
                    native_fn(arg0, arg1)
                }
                _ => return Ok(false),
            };

            self.stack.truncate(self.stack.len() - arg_count - 1);
            self.push(Value::number(result_num));

            return Ok(true);
        }

        Ok(false)
    }
}
