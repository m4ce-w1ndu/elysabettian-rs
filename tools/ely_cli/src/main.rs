use std::fs;
use std::path::PathBuf;
use std::process;

use clap::Parser;
use ely_compiler::compiler::compile;
use ely_core::interner::StringInterner;
use ely_vm::vm::{InterpretResult, VM};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

#[derive(Parser, Debug)]
#[command(
    name = "ely",
    author = "m4ce-w1ndu",
    version = "0.1.0",
    about = "Elysabettian programming language runtime and REPL"
)]
struct Cli {
    /// Path to the .ely script to run
    #[arg(value_name = "SCRIPT")]
    script: Option<PathBuf>,

    /// One-liner command script to execute
    #[arg(short = 'c', long = "command", value_name = "CODE")]
    command: Option<String>,
}

fn run_source(source: &str, vm: &mut VM, interner: &mut StringInterner) -> InterpretResult {
    let chunk = match compile(source, interner) {
        Some(chunk) => chunk,
        None => return InterpretResult::CompileError,
    };

    vm.interpret(&chunk)
}

fn run_file(path: &PathBuf, vm: &mut VM, interner: &mut StringInterner) {
    let source = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading file '{:?}': {}", path, err);
            process::exit(74);
        }
    };

    match run_source(&source, vm, interner) {
        InterpretResult::Ok => (),
        InterpretResult::CompileError => process::exit(65),
        InterpretResult::RuntimeError => process::exit(70),
    }
}

fn run_repl(vm: &mut VM, interner: &mut StringInterner) {
    println!("Elysabettian REPL (v0.1.0)");
    println!("Type 'exit' or press Ctrl+D to exit.\n");

    let mut rl = match DefaultEditor::new() {
        Ok(editor) => editor,
        Err(err) => {
            eprintln!("Failed to initialize interactive line editor: {}", err);
            return;
        }
    };

    loop {
        let readline = rl.readline("ely> ");
        match readline {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed == "exit" || trimmed == "quit" {
                    break;
                }
                if trimmed.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(line.as_str());
                run_source(&line, vm, interner);
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
            }
            Err(ReadlineError::Eof) => {
                println!();
                break;
            }
            Err(err) => {
                eprintln!("Readline error: {:?}", err);
                break;
            }
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let mut vm = VM::new();
    let mut interner = StringInterner::new();

    if let Some(cmd) = cli.command {
        match run_source(&cmd, &mut vm, &mut interner) {
            InterpretResult::Ok => (),
            InterpretResult::CompileError => process::exit(65),
            InterpretResult::RuntimeError => process::exit(70),
        }
    } else if let Some(path) = cli.script {
        run_file(&path, &mut vm, &mut interner);
    } else {
        run_repl(&mut vm, &mut interner);
    }
}
