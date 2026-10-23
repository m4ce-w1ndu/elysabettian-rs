#!/usr/bin/env python3
"""
Elysabettian Performance Benchmarking Tool
Compares VM Interpreter vs Native Cranelift JIT execution speed.
"""

import os
import subprocess
import time
import sys

CLI_PATH = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "target", "release", "ely_cli"))
SCRIPT_PATH = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "examples", "bench_loop.ely"))

def build_release():
    print("[*] Compiling ely_cli in release mode (cargo build --release)...")
    res = subprocess.run(["cargo", "build", "--release", "-p", "ely_cli"], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if res.returncode != 0:
        print("[!] Build failed:", res.stderr.decode())
        sys.exit(1)
    print("[+] Build successful!\n")

def run_benchmark(mode_name, args, iterations=3):
    durations = []
    output = None
    cmd = [CLI_PATH] + args + [SCRIPT_PATH]

    for i in range(iterations):
        start = time.perf_counter()
        proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        elapsed = time.perf_counter() - start
        if proc.returncode != 0:
            print(f"[!] Error in {mode_name}:", proc.stderr)
            return None, None
        durations.append(elapsed)
        output = proc.stdout.strip()

    avg_time = sum(durations) / len(durations)
    return avg_time, output

def main():
    build_release()

    print(f"[*] Running benchmark on: {SCRIPT_PATH}")
    print("[*] Benchmark task: Loop with 2,000,000 iterations and accumulator\n")

    print("-" * 55)
    print(f"{'Execution Mode':<25} | {'Average Time (s)':<15} | {'Result'}")
    print("-" * 55)

    # 1. Pure VM Interpreter (--no-jit)
    vm_time, vm_out = run_benchmark("VM Interpreter", ["--no-jit"])
    print(f"{'VM Interpreter (--no-jit)':<25} | {vm_time:<15.4f} | {vm_out}")

    # 2. Native Cranelift JIT (--jit)
    jit_time, jit_out = run_benchmark("Cranelift JIT", ["--jit"])
    print(f"{'Native JIT (--jit)':<25} | {jit_time:<15.4f} | {jit_out}")
    print("-" * 55)

    if vm_time and jit_time:
        speedup = vm_time / jit_time
        print(f"\n🚀 Cranelift JIT is {speedup:.2f}x faster than the VM interpreter!")

if __name__ == "__main__":
    main()
