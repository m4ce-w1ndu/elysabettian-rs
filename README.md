# Elysabettian (Rust Implementation)

A modern, high-performance runtime and Just-In-Time (JIT) compiler for the **Elysabettian** scripting language, rewritten in Rust.

---

## Overview

Elysabettian is a lightweight, dynamically typed scripting language featuring a clean, JavaScript-like syntax. This repository represents the next-generation implementation of the Elysabettian runtime, transitioning from the original C++17 virtual machine to a memory-safe, modular Rust architecture featuring a multi-tier execution engine and a native JIT compiler powered by [Cranelift](https://cranelift.dev/).

### Key Goals & Highlights

- **Safety & Predictability:** Eliminates manual memory bugs by leveraging Rust's memory safety guarantees and modular crate structure.
- **True JIT Compilation:** Generates native machine code (x86_64 / AArch64) using Cranelift, turning hot paths, loops, and numeric arithmetic into direct CPU instructions.
- **Compact Value Representation:** Engineered around IEEE-754 64-bit NaN-boxing (`sizeof(Value) == 8`) to maximize CPU L1/L2 cache locality.
- **Multi-Tier Execution:** Seamlessly runs through a fast bytecode interpreter while automatically promoting hot functions to JIT machine code at runtime.
- **Developer & Compiler Tooling:** First-class inspection flags (`--dump-ir`, `--dump-asm`), interactive REPL with history, and automated benchmarking.

---

## Workspace Architecture

The repository is organized as a modular Cargo workspace:

```text
elysabettian-rs/
├── libs/
│   ├── ely_core         # Core primitives, 64-bit NaN-boxed values, opcodes, and bytecode chunks
│   ├── ely_compiler     # Zero-allocation lexer, Pratt parser, and single-pass bytecode emitter
│   ├── ely_vm           # Stack-based runtime VM, object heap, and hot-path JIT tier-up
│   └── ely_jit          # Cranelift SSA translation, CFG construction, and native JIT execution
├── tools/
│   ├── ely_cli          # Modular CLI driver, interactive REPL, and code generation flags
│   └── benchmark.py     # Automated performance instrumentation comparing VM vs JIT
├── examples/            # Benchmark scripts, language fixtures, and tests
└── docs/                # Architecture guides, JIT internals, and opcode specifications
```

---

## Documentation Directory

Comprehensive design specifications and architectural documentation are available in [`docs/`](docs/):

| Document | Description |
| :--- | :--- |
| [System Architecture](docs/architecture.md) | End-to-end design, compilation pipeline, workspace layout, and multi-tier execution model. |
| [Cranelift JIT Specification](docs/jit_compiler.md) | JIT engine internals, SSA lowering, CFG blocks, runtime trampolines, `--dump-ir`, and `--dump-asm`. |
| [CLI & Tooling Guide](docs/cli_tooling.md) | Command-line driver, interactive REPL options, and performance benchmarking instructions. |
| [Bytecode & Opcode Reference](docs/opcodes_specification.md) | Complete instruction set architecture (ISA), stack transition rules, and closure capture semantics. |
| [NaN-Boxing Value Specification](docs/nan_boxing_specification.md) | IEEE-754 64-bit layout, bitmasks, payload packing, and type guards. |

---

## Prerequisites & Installation

- **Rust toolchain:** Rust 1.75+ (or latest stable) with Cargo.
- **Platform support:** Linux, macOS, and Windows (x86_64, AArch64).

```shell
cargo --version
rustc --version
```

---

## Quickstart & CLI Usage

### Build the Workspace

```shell
cargo build --workspace
```

### Run Unit Tests

```shell
cargo test --workspace
```

### Launch Interactive REPL

```shell
# Standard bytecode VM REPL
cargo run -p ely_cli

# Pure native Cranelift JIT REPL
cargo run -p ely_cli -- --jit
```

### Execute a Script

```shell
# Run via VM (with automatic dynamic hot-path tier-up)
cargo run -p ely_cli -- examples/bench_loop.ely

# Force pure Cranelift JIT execution
cargo run -p ely_cli -- --jit examples/bench_loop.ely

# Force pure bytecode VM execution (disables dynamic JIT)
cargo run -p ely_cli -- --no-jit examples/bench_loop.ely
```

### Inspect Intermediate Representation & Assembly

```shell
# Dump Cranelift SSA IR
cargo run -p ely_cli -- --jit --dump-ir examples/bench_loop.ely

# Dump native machine code disassembly (VCode)
cargo run -p ely_cli -- --jit --dump-asm examples/bench_loop.ely
```

---

## Scientific Performance Benchmarking

The project includes an automated statistical benchmarking and visualization suite in [`tools/benchmark.py`](tools/benchmark.py) evaluating diverse mathematical and algorithmic workloads:

```shell
python3 tools/benchmark.py
```

### Evaluation Workloads
1. **Accumulator Loop** (`bench_loop.ely`): 2,000,000 iterations measuring basic loop overhead and variable updates.
2. **Transcendental Math** (`bench_arithmetic.ely`): 2,000,000 iterations testing floating-point operations ($+, -, *, /$ with $\pi$ and $e$).
3. **Fibonacci Modulo** (`bench_fibonacci.ely`): 1,000,000 steps testing register-shifting sequence accumulation.
4. **Leibniz Pi Series** (`bench_leibniz_pi.ely`): 5,000,000 iterations evaluating alternating signs, division, and numerical precision.
5. **Collatz Conjecture** (`bench_collatz.ely`): 25,000 sequences testing conditionals, nested loops, and bitwise operations (`& 1`, `>> 1`).

### Benchmark Results (2 Warmup Runs, 5 Timed Evaluations)

| Workload | VM Mean Time (s) | JIT Mean Time (s) | Speedup Factor | Numerical Parity |
| :--- | :---: | :---: | :---: | :---: |
| **Accumulator Loop** | 0.5859s ± 0.0079 | 0.2309s ± 0.0007 | **2.54x** | Verified |
| **Transcendental Math** | 0.6117s ± 0.0821 | 0.1745s ± 0.0020 | **3.51x** | Verified |
| **Fibonacci Modulo** | 0.5781s ± 0.0029 | 0.3851s ± 0.0026 | **1.50x** | Verified |
| **Leibniz Pi Series** | 1.7856s ± 0.0021 | 0.8623s ± 0.0222 | **2.07x** | Verified |
| **Collatz Conjecture** | 0.7706s ± 0.0126 | 0.4418s ± 0.0038 | **1.74x** | Verified |

Detailed reports and graphical charts generated by `matplotlib` are saved in [`tools/results/`](tools/results/):
- **Execution Times:** [`tools/results/benchmark_execution_times.png`](tools/results/benchmark_execution_times.png)
- **Speedup Factors:** [`tools/results/benchmark_speedup.png`](tools/results/benchmark_speedup.png)
- **Full Markdown Report:** [`tools/results/BENCHMARK_REPORT.md`](tools/results/BENCHMARK_REPORT.md)

---

## Roadmap

- [x] Multi-crate workspace scaffold and core configuration (`ely_core`).
- [x] Zero-allocation Tokenizer / Lexer with source span tracking (`ely_compiler`).
- [x] Single-pass Pratt Parser and Bytecode Emitter (`ely_compiler`).
- [x] Stack-based Virtual Machine and Object Heap Runtime (`ely_vm`).
- [x] Cranelift JIT compilation tier for native machine execution (`ely_jit`).
- [x] Dynamic hot-path invocation tier-up from VM to JIT (`ely_vm` + `ely_jit`).
- [x] Modular CLI with REPL, inspection flags (`--dump-ir`, `--dump-asm`), and Python benchmark suite.
- [ ] Garbage Collection tracing (Mark & Sweep / Arena GC).
- [ ] Enhanced standard library modules (`math`, `io`, `os`).

---

## License

This project is licensed under the [MIT License](LICENSE).
