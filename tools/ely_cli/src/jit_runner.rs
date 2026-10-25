use ely_core::chunk::Chunk;
use ely_core::opcode::Opcode;
use ely_jit::compiler::JitCompiler;
use ely_jit::engine::JitEngine;
use ely_jit::runtime::{append_jit_strings, register_jit_function};
use std::sync::atomic::{AtomicUsize, Ordering};

static JIT_EVAL_COUNTER: AtomicUsize = AtomicUsize::new(0);
static JIT_FUNC_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Recursively registers strings and compiles nested functions for JIT.
fn prepare_jit_chunk(chunk: &mut Chunk, compiler: &mut JitCompiler) -> Result<(), String> {
    let base_string_idx = append_jit_strings(&chunk.string_constants);

    let mut func_ptrs = Vec::with_capacity(chunk.function_constants.len());
    for (i, f) in chunk.function_constants.iter_mut().enumerate() {
        prepare_jit_chunk(&mut f.chunk, compiler)?;
        let fn_name = f.name.clone().unwrap_or_else(|| format!("jit_fn_{}", i));
        let ptr = compiler.compile_function(&f.chunk, f.arity, &fn_name)?;
        func_ptrs.push(ptr);
    }

    let mut func_ids = Vec::with_capacity(func_ptrs.len());
    for ptr in func_ptrs {
        let func_id = JIT_FUNC_COUNTER.fetch_add(1, Ordering::SeqCst);
        register_jit_function(func_id, ptr);
        func_ids.push(func_id);
    }

    // Patch chunk.constants so that Opcode::Constant with string and Opcode::Closure
    // point to their actual global runtime IDs in JIT_STRINGS and JIT_FUNCTIONS
    let mut ip = 0;
    while ip < chunk.code.len() {
        if let Ok(opcode) = Opcode::try_from(chunk.code[ip]) {
            ip += 1;
            match opcode {
                Opcode::Constant => {
                    let const_idx = chunk.code[ip] as usize;
                    ip += 1;
                    if let Some(c) = chunk.constants.get_mut(const_idx) {
                        if c.is_obj() {
                            let orig_idx = c.as_obj() as usize;
                            *c = ely_core::value::Value::obj((base_string_idx + orig_idx) as u64);
                        }
                    }
                }
                Opcode::Closure => {
                    let const_idx = chunk.code[ip] as usize;
                    ip += 1;
                    if let Some(c) = chunk.constants.get_mut(const_idx) {
                        if c.is_obj() {
                            let orig_idx = c.as_obj() as usize;
                            if let Some(&real_id) = func_ids.get(orig_idx) {
                                *c = ely_core::value::Value::obj(real_id as u64);
                            }
                        }
                    }
                }
                Opcode::GetLocal
                | Opcode::SetLocal
                | Opcode::GetGlobal
                | Opcode::DefineGlobal
                | Opcode::SetGlobal
                | Opcode::Call
                | Opcode::Class
                | Opcode::GetProperty
                | Opcode::SetProperty
                | Opcode::Method
                | Opcode::Invoke
                | Opcode::SuperInvoke
                | Opcode::Import => {
                    ip += 1;
                }
                Opcode::Jump | Opcode::JumpIfFalse | Opcode::Loop => {
                    ip += 2;
                }
                _ => {}
            }
        } else {
            ip += 1;
        }
    }

    Ok(())
}

/// Directly compiles and executes a bytecode chunk using Cranelift JIT natively on the CPU.
pub fn run_jit(
    chunk: &Chunk,
    engine: &mut JitEngine,
    dump_ir: bool,
    dump_asm: bool,
) -> Result<f64, String> {
    let mut compiler = JitCompiler::new(engine);
    compiler.dump_ir = dump_ir;
    compiler.dump_asm = dump_asm;

    let mut executable_chunk = chunk.clone();
    prepare_jit_chunk(&mut executable_chunk, &mut compiler)?;

    let eval_idx = JIT_EVAL_COUNTER.fetch_add(1, Ordering::SeqCst);
    let eval_name = format!("jit_eval_{}", eval_idx);
    let code_ptr = compiler.compile_arithmetic(&executable_chunk, &eval_name)?;

    let native_func: extern "C" fn() -> f64 = unsafe { std::mem::transmute(code_ptr) };
    let result = native_func();

    Ok(result)
}
