use ely_core::chunk::Chunk;
use ely_jit::compiler::JitCompiler;
use ely_jit::engine::JitEngine;
use ely_jit::runtime::append_jit_strings;

/// Directly compiles and executes a bytecode chunk using Cranelift JIT natively on the CPU.
pub fn run_jit(chunk: &Chunk, dump_ir: bool, dump_asm: bool) -> Result<f64, String> {
    // Register any string constants in the JIT runtime string pool
    append_jit_strings(&chunk.string_constants);

    let mut engine = JitEngine::new().map_err(|e| format!("Failed to create JIT engine: {}", e))?;
    let mut compiler = JitCompiler::new(&mut engine);
    compiler.dump_ir = dump_ir;
    compiler.dump_asm = dump_asm;

    let code_ptr = compiler.compile_arithmetic(chunk, "main_jit")?;

    let native_func: extern "C" fn() -> f64 = unsafe { std::mem::transmute(code_ptr) };
    let result = native_func();

    Ok(result)
}
