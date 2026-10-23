use std::path::PathBuf;
use clap::Parser;

/// Command line interface arguments for Elysabettian.
#[derive(Parser, Debug)]
#[command(
    name = "ely",
    author = "m4ce-w1ndu",
    version = "0.1.0",
    about = "Elysabettian programming language runtime, JIT compiler, and REPL"
)]
pub struct Cli {
    /// Path to the .ely script to run
    #[arg(value_name = "SCRIPT")]
    pub script: Option<PathBuf>,

    /// One-liner command script to execute
    #[arg(short = 'c', long = "command", value_name = "CODE")]
    pub command: Option<String>,

    /// Execute the entire script directly via native Cranelift JIT (bypassing VM interpreter)
    #[arg(long = "jit")]
    pub jit: bool,

    /// Disable dynamic JIT tier-up in VM interpreter
    #[arg(long = "no-jit")]
    pub no_jit: bool,
}
