use ely_core::interner::StringInterner;
use ely_vm::vm::VM;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

use crate::runner::run_source;

/// Starts the interactive Read-Eval-Print Loop for Elysabettian.
pub fn run_repl(
    vm: &mut VM,
    interner: &mut StringInterner,
    jit_only: bool,
    dump_ir: bool,
    dump_asm: bool,
) {
    let mode = if jit_only {
        "JIT Native Mode"
    } else {
        "VM Tier-up Mode"
    };
    println!(
        "Elysabettian REPL (v{}) [{}]",
        env!("CARGO_PKG_VERSION"),
        mode
    );
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
                run_source(&line, vm, interner, jit_only, dump_ir, dump_asm);
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
