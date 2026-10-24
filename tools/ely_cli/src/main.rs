mod args;
mod jit_runner;
mod repl;
mod runner;

use std::process;

use clap::Parser;
use ely_core::interner::StringInterner;
use ely_vm::vm::{InterpretResult, VM};

use crate::args::Cli;
use crate::repl::run_repl;
use crate::runner::{run_file, run_source};

fn main() {
    let cli = Cli::parse();
    let mut vm = VM::new();
    let mut interner = StringInterner::new();
    vm.register_stdlib(&mut interner);

    if cli.no_jit {
        vm.jit_engine = None;
    }

    if let Some(cmd) = cli.command {
        match run_source(
            &cmd,
            &mut vm,
            &mut interner,
            cli.jit,
            cli.dump_ir,
            cli.dump_asm,
        ) {
            InterpretResult::Ok => (),
            InterpretResult::CompileError => process::exit(65),
            InterpretResult::RuntimeError => process::exit(70),
        }
    } else if let Some(path) = cli.script {
        run_file(
            &path,
            &mut vm,
            &mut interner,
            cli.jit,
            cli.dump_ir,
            cli.dump_asm,
        );
    } else {
        run_repl(&mut vm, &mut interner, cli.jit, cli.dump_ir, cli.dump_asm);
    }
}
