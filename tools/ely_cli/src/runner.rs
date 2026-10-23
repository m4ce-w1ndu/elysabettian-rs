use std::fs;
use std::path::PathBuf;
use std::process;

use ely_compiler::compiler::compile;
use ely_core::interner::StringInterner;
use ely_vm::vm::{InterpretResult, VM};

use crate::jit_runner::run_jit;

/// Runs Elysabettian source code, either entirely through native Cranelift JIT or via the VM.
pub fn run_source(
    source: &str,
    vm: &mut VM,
    interner: &mut StringInterner,
    jit_only: bool,
    dump_ir: bool,
    dump_asm: bool,
) -> InterpretResult {
    let chunk = match compile(source, interner) {
        Some(chunk) => chunk,
        None => return InterpretResult::CompileError,
    };

    if jit_only {
        match run_jit(&chunk, dump_ir, dump_asm) {
            Ok(_) => InterpretResult::Ok,
            Err(err) => {
                eprintln!("JIT Execution Error: {}", err);
                InterpretResult::RuntimeError
            }
        }
    } else {
        vm.interpret(&chunk)
    }
}

/// Reads a source script file and executes it.
pub fn run_file(
    path: &PathBuf,
    vm: &mut VM,
    interner: &mut StringInterner,
    jit_only: bool,
    dump_ir: bool,
    dump_asm: bool,
) {
    let source = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading file '{:?}': {}", path, err);
            process::exit(74);
        }
    };

    match run_source(&source, vm, interner, jit_only, dump_ir, dump_asm) {
        InterpretResult::Ok => (),
        InterpretResult::CompileError => process::exit(65),
        InterpretResult::RuntimeError => process::exit(70),
    }
}
