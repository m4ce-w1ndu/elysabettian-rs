use ely_core::chunk::Chunk;
use ely_jit::compiler::JitCompiler;
use ely_jit::engine::JitEngine;
use ely_jit::runtime::{append_jit_strings, register_jit_function};
use std::sync::atomic::{AtomicUsize, Ordering};

static JIT_EVAL_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Recursively registers strings and compiles nested functions for JIT.
fn prepare_jit_functions(chunk: &Chunk, compiler: &mut JitCompiler) -> Result<(), String> {
    append_jit_strings(&chunk.string_constants);

    for (i, f) in chunk.function_constants.iter().enumerate() {
        prepare_jit_functions(&f.chunk, compiler)?;
        let fn_name = f.name.clone().unwrap_or_else(|| format!("jit_fn_{}", i));
        let ptr = compiler.compile_function(&f.chunk, f.arity, &fn_name)?;
        register_jit_function(i, ptr);
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

    prepare_jit_functions(chunk, &mut compiler)?;

    let eval_idx = JIT_EVAL_COUNTER.fetch_add(1, Ordering::SeqCst);
    let eval_name = format!("jit_eval_{}", eval_idx);
    let code_ptr = compiler.compile_arithmetic(chunk, &eval_name)?;

    let native_func: extern "C" fn() -> f64 = unsafe { std::mem::transmute(code_ptr) };
    let result = native_func();

    Ok(result)
}
