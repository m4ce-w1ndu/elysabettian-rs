#!/usr/bin/env python3
"""
Elysabettian Scientific Benchmarking Suite
Performs multi-iteration statistical analysis and visualization comparing
Bytecode Virtual Machine interpreter vs Native Cranelift JIT compiler.
"""

import os
import sys
import time
import math
import json
import subprocess
from statistics import mean, median, stdev
from pathlib import Path

# Paths
ROOT_DIR = Path(__file__).resolve().parent.parent
CLI_BIN = ROOT_DIR / "target" / "release" / "ely_cli"
EXAMPLES_DIR = ROOT_DIR / "examples"
RESULTS_DIR = ROOT_DIR / "tools" / "results"

BENCHMARKS = [
    {
        "id": "bench_loop",
        "file": "bench_loop.ely",
        "name": "Accumulator Loop",
        "desc": "Simple numeric counter & accumulator (2,000,000 iters)",
    },
    {
        "id": "bench_arithmetic",
        "file": "bench_arithmetic.ely",
        "name": "Transcendental Math",
        "desc": "Floating-point multiply/divide with Pi & e (2,000,000 iters)",
    },
    {
        "id": "bench_fibonacci",
        "file": "bench_fibonacci.ely",
        "name": "Fibonacci Modulo",
        "desc": "Iterative Fibonacci sequence with register shifts (1,000,000 iters)",
    },
    {
        "id": "bench_leibniz_pi",
        "file": "bench_leibniz_pi.ely",
        "name": "Leibniz Pi Series",
        "desc": "Alternating infinite series approximation of Pi (5,000,000 iters)",
    },
    {
        "id": "bench_collatz",
        "file": "bench_collatz.ely",
        "name": "Collatz Conjecture",
        "desc": "3n + 1 trajectory steps using bitwise shifts (25,000 ranges)",
    },
]

def build_cli_release():
    print("[*] Compiling ely_cli in release mode (cargo build --release -p ely_cli)...")
    res = subprocess.run(
        ["cargo", "build", "--release", "-p", "ely_cli"],
        cwd=ROOT_DIR,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if res.returncode != 0:
        print("[!] Build failed:\n", res.stderr)
        sys.exit(1)
    print("[+] Release binary ready at:", CLI_BIN, "\n")

def run_single(cmd):
    start = time.perf_counter()
    proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    elapsed = time.perf_counter() - start
    if proc.returncode != 0:
        raise RuntimeError(f"Command failed (exit {proc.returncode}): {proc.stderr.strip()}")
    return elapsed, proc.stdout.strip()

def benchmark_workload(bench_path, mode_flag, warmup_runs=2, measured_runs=5):
    cmd = [str(CLI_BIN), mode_flag, str(bench_path)]

    # Warmup
    last_output = None
    for _ in range(warmup_runs):
        _, last_output = run_single(cmd)

    # Measured runs
    timings = []
    for _ in range(measured_runs):
        elapsed, last_output = run_single(cmd)
        timings.append(elapsed)

    std_val = stdev(timings) if len(timings) > 1 else 0.0
    return {
        "timings": timings,
        "mean": mean(timings),
        "median": median(timings),
        "stdev": std_val,
        "min": min(timings),
        "max": max(timings),
        "output": last_output,
    }

def generate_charts(results):
    try:
        import matplotlib.pyplot as plt
        import numpy as np
    except ImportError:
        print("[!] matplotlib or numpy not installed. Skipping graphical chart generation.")
        return

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    labels = [b["name"] for b in results]
    vm_means = [b["vm"]["mean"] for b in results]
    vm_errs = [b["vm"]["stdev"] for b in results]
    jit_means = [b["jit"]["mean"] for b in results]
    jit_errs = [b["jit"]["stdev"] for b in results]
    speedups = [b["speedup"] for b in results]

    # Chart 1: Absolute Execution Times (Grouped Bar Chart)
    x = np.arange(len(labels))
    width = 0.35

    fig, ax = plt.subplots(figsize=(10, 6))
    rects1 = ax.bar(x - width/2, vm_means, width, yerr=vm_errs, capsize=4,
                    label='VM Bytecode Interpreter (--no-jit)', color='#e05d44', alpha=0.9)
    rects2 = ax.bar(x + width/2, jit_means, width, yerr=jit_errs, capsize=4,
                    label='Native Cranelift JIT (--jit)', color='#007ec6', alpha=0.9)

    ax.set_ylabel('Execution Time (seconds) - lower is better', fontsize=12)
    ax.set_title('Elysabettian Performance: VM vs Cranelift JIT (Mean ± StdDev)', fontsize=14, fontweight='bold')
    ax.set_xticks(x)
    ax.set_xticklabels(labels, rotation=15, ha='right', fontsize=11)
    ax.legend(fontsize=11)
    ax.grid(axis='y', linestyle='--', alpha=0.5)

    # Add labels on bars
    def autolabel(rects):
        for rect in rects:
            height = rect.get_height()
            ax.annotate(f'{height:.3f}s',
                        xy=(rect.get_x() + rect.get_width() / 2, height),
                        xytext=(0, 3),
                        textcoords="offset points",
                        ha='center', va='bottom', fontsize=9)
    autolabel(rects1)
    autolabel(rects2)

    fig.tight_layout()
    chart1_path = RESULTS_DIR / "benchmark_execution_times.png"
    plt.savefig(chart1_path, dpi=200)
    plt.close()
    print(f"[+] Execution time chart saved to: {chart1_path}")

    # Chart 2: Speedup Factor (Horizontal Bar Chart)
    fig, ax = plt.subplots(figsize=(9, 5))
    y_pos = np.arange(len(labels))
    bars = ax.barh(y_pos, speedups, color='#2ca02c', alpha=0.85, height=0.55)

    ax.set_xlabel('Speedup Factor (VM Time / JIT Time) - higher is better', fontsize=12)
    ax.set_title('Cranelift JIT Acceleration Factor by Workload', fontsize=14, fontweight='bold')
    ax.set_yticks(y_pos)
    ax.set_yticklabels(labels, fontsize=11)
    ax.axvline(1.0, color='red', linestyle='--', linewidth=1.5, label='Baseline (1.0x)')
    ax.grid(axis='x', linestyle='--', alpha=0.5)

    for bar in bars:
        width = bar.get_width()
        ax.annotate(f'{width:.2f}x',
                    xy=(width, bar.get_y() + bar.get_height() / 2),
                    xytext=(5, 0),
                    textcoords="offset points",
                    ha='left', va='center', fontsize=10, fontweight='bold')

    ax.legend(loc='lower right')
    fig.tight_layout()
    chart2_path = RESULTS_DIR / "benchmark_speedup.png"
    plt.savefig(chart2_path, dpi=200)
    plt.close()
    print(f"[+] Speedup chart saved to: {chart2_path}")

def generate_report(results):
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    report_path = RESULTS_DIR / "BENCHMARK_REPORT.md"
    json_path = RESULTS_DIR / "benchmark_data.json"

    with open(json_path, "w") as f:
        json.dump(results, f, indent=2)

    with open(report_path, "w") as f:
        f.write("# Elysabettian Scientific Benchmark Report\n\n")
        f.write("Evaluation of runtime performance comparing Bytecode Virtual Machine (`--no-jit`) with Cranelift Native JIT (`--jit`).\n\n")
        f.write("### Methodology\n")
        f.write("- **Hardware Platform:** Host CPU execution via Cranelift Native ISA.\n")
        f.write("- **Execution Rounds:** 2 warmup iterations followed by 5 timed evaluation runs per workload.\n")
        f.write(r"- **Statistical Metrics:** Sample Mean ($\mu$), Sample Standard Deviation ($\sigma$), and Speedup Factor ($\mu_{VM} / \mu_{JIT}$)." + "\n")
        f.write("- **Verification:** Bit-exact string comparison of output values between VM and JIT executions.\n\n")
        f.write("### Benchmark Results Summary\n\n")
        f.write("| Workload | Description | VM Mean (s) | JIT Mean (s) | Speedup | Result Parity |\n")
        f.write("| :--- | :--- | :---: | :---: | :---: | :---: |\n")

        for b in results:
            parity = "Verified" if b["parity"] else "FAILED"
            f.write(f"| **{b['name']}** | {b['desc']} | {b['vm']['mean']:.4f}s ± {b['vm']['stdev']:.4f} | {b['jit']['mean']:.4f}s ± {b['jit']['stdev']:.4f} | **{b['speedup']:.2f}x** | {parity} |\n")

        f.write("\n### Visual Performance Charts\n\n")
        f.write("![Execution Times](benchmark_execution_times.png)\n\n")
        f.write("![Speedup Factors](benchmark_speedup.png)\n")

    print(f"[+] Markdown report saved to: {report_path}")
    print(f"[+] JSON raw data saved to: {json_path}")

def main():
    build_cli_release()
    print("=" * 80)
    print(f"{'Elysabettian Benchmark Suite':^80}")
    print(f"{'5 Workloads | 2 Warmup Runs | 5 Timed Evaluations':^80}")
    print("=" * 80 + "\n")

    all_results = []

    for b in BENCHMARKS:
        bench_path = EXAMPLES_DIR / b["file"]
        if not bench_path.exists():
            print(f"[!] Warning: {bench_path} not found, skipping.")
            continue

        print(f"[*] Running workload: {b['name']} ({b['file']})")
        print(f"    Description: {b['desc']}")

        vm_stat = benchmark_workload(bench_path, "--no-jit")
        jit_stat = benchmark_workload(bench_path, "--jit")

        parity = (vm_stat["output"] == jit_stat["output"])
        speedup = vm_stat["mean"] / jit_stat["mean"] if jit_stat["mean"] > 0 else 0.0

        print(f"    VM Mean:  {vm_stat['mean']:.4f}s ± {vm_stat['stdev']:.4f}s")
        print(f"    JIT Mean: {jit_stat['mean']:.4f}s ± {jit_stat['stdev']:.4f}s")
        print(f"    Speedup:  {speedup:.2f}x (Parity: {'MATCH' if parity else 'MISMATCH'})\n")

        all_results.append({
            "id": b["id"],
            "name": b["name"],
            "desc": b["desc"],
            "file": b["file"],
            "vm": vm_stat,
            "jit": jit_stat,
            "parity": parity,
            "speedup": speedup,
        })

    # Print summary table
    print("=" * 80)
    print(f"{'Workload':<20} | {'VM Time (s)':<15} | {'JIT Time (s)':<15} | {'Speedup':<10} | {'Parity'}")
    print("-" * 80)
    for res in all_results:
        p_str = "OK" if res["parity"] else "FAIL"
        print(f"{res['name']:<20} | {res['vm']['mean']:>7.4f}s ± {res['vm']['stdev']:<5.4f} | {res['jit']['mean']:>7.4f}s ± {res['jit']['stdev']:<5.4f} | {res['speedup']:>6.2f}x    | {p_str}")
    print("=" * 80 + "\n")

    generate_charts(all_results)
    generate_report(all_results)

if __name__ == "__main__":
    main()
