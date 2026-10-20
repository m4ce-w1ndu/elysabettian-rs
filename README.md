# Elysabettian (Rust Implementation)

A modern, high-performance runtime and Just-In-Time (JIT) compiler for the **Elysabettian** scripting language, rewritten in Rust.

---

## Overview

Elysabettian is a lightweight, dynamically typed scripting language featuring a clean, JavaScript-like syntax. This repository represents the next-generation implementation of the Elysabettian runtime, transitioning from the original C++17 virtual machine to a memory-safe, modular Rust architecture featuring a multi-tier execution engine and a native JIT compiler powered by [Cranelift](https://cranelift.dev/).

### Key Goals & Highlights

- **Safety & Predictability:** Eliminates manual memory bugs and cyclic reference leaks by leveraging Rust's ownership model and arena-based garbage collection.
- **True JIT Compilation:** Replaces pure bytecode interpretation with dynamic machine code generation using Cranelift, optimizing hot loops and numeric paths directly into native instructions.
- **Compact Value Representation:** Engineered around cache-friendly value representations (tagged unions and NaN-boxing) to drastically reduce memory footprint and CPU cache thrashing.
- **Modular Workspace Architecture:** Clear separation of concerns between compilation, bytecode generation, runtime execution, and native code generation.

---

## Workspace Architecture

The project is structured as a multi-package Cargo workspace:

```text
elysabettian-rs/
├── libs/
│   ├── ely_core         # Shared primitive types, bytecode definitions, opcodes, and chunks
│   ├── ely_compiler     # Lexer, Pratt parser, and bytecode compiler
│   ├── ely_vm           # Stack-based runtime, object allocators, and standard libraries
│   └── ely_jit          # Cranelift IR translator, JIT engine, and execution trampolines
├── tools/
│   └── ely_cli          # Interactive REPL and command-line file execution driver
└── examples/            # Language fixtures, benchmarks, and example programs
```

---

## Prerequisites

- **Rust toolchain:** Rust 1.75+ (or latest stable) with Cargo.
- **Platform support:** Linux, macOS, and Windows (x86_64, AArch64).

To verify your Rust environment:

```shell
cargo --version
rustc --version
```

---

## Documentation

Project specifications and design documentation are located in the [`docs/`](docs/) directory:

- [Bytecode & Opcode Specification](docs/opcodes_specification.md): Detailed instruction set reference, operand encoding, stack transition schemas, and closure capture mechanics.
- [NaN-Boxing Value Specification](docs/nan_boxing_specification.md): Complete IEEE-754 64-bit layout, bitmasks, pointer packing, type guards, and canonicalization.

---

## Building & Testing

### Build the entire workspace

```shell
cargo build --workspace
```

### Run tests

```shell
cargo test --workspace
```

### Launch the REPL

```shell
cargo run -p ely_cli
```

### Execute a script

```shell
cargo run -p ely_cli -- path/to/script.ely
```

---

## Roadmap

1. [x] Multi-crate workspace scaffold and core configuration.
2. [ ] Zero-allocation Tokenizer / Lexer with accurate source span tracking.
3. [ ] Single-pass Pratt Parser and Bytecode Emitter (`ely_compiler`).
4. [ ] Stack-based Virtual Machine and Object Runtime (`ely_vm`).
5. [ ] Cranelift JIT compilation tier for hot methods and arithmetic loops (`ely_jit`).
6. [ ] Enhanced standard library (`math`, `stdio`, dynamic arrays).

---

## License

This project is licensed under the [MIT License](LICENSE).
