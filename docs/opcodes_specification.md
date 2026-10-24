# Elysabettian Bytecode Specification

This document details the instruction set architecture (ISA) of the Elysabettian Virtual Machine.
Each opcode is an 8-bit unsigned integer (`u8`) executed by the stack-based interpreter or compiled to native machine code by the Cranelift JIT engine.

---

## 1. Instruction Set Architecture Overview

* **Evaluation Model:** Stack-based virtual machine (LIFO operand evaluation).
* **Endianness:** Big-endian 16-bit jump targets (`offset_high << 8 | offset_low`).
* **Operands:** Instructions can take inline bytecode operands (constant indices, stack slot indices, or jump offsets) placed immediately after the opcode.

---

## 2. Complete Opcode Reference Table

| Hex | Dec | Opcode | Inline Operands | Stack Transition `[before] -> [after]` | Description |
| :---: | :---: | :--- | :--- | :--- | :--- |
| `0x00` | 0 | `Constant` | `[const_index: u8]` | `[...] -> [..., value]` | Pushes `constants[const_index]` onto the stack. |
| `0x01` | 1 | `Nop` | None | `[...] -> [...]` | No operation. Advances the instruction pointer. |
| `0x02` | 2 | `True` | None | `[...] -> [..., true]` | Pushes boolean `true` onto the stack. |
| `0x03` | 3 | `False` | None | `[...] -> [..., false]` | Pushes boolean `false` onto the stack. |
| `0x04` | 4 | `Pop` | None | `[..., value] -> [...]` | Discards the top stack value. |
| `0x05` | 5 | `GetLocal` | `[slot: u8]` | `[...] -> [..., stack[frame_base + slot]]` | Reads local variable from current call frame. |
| `0x06` | 6 | `GetGlobal` | `[name_index: u8]` | `[...] -> [..., value]` | Looks up global variable by constant name string. |
| `0x07` | 7 | `DefineGlobal` | `[name_index: u8]` | `[..., value] -> [...]` | Defines/binds a global variable with name. Pops value. |
| `0x08` | 8 | `SetLocal` | `[slot: u8]` | `[..., value] -> [..., value]` | Updates local slot. Leaves value on stack. |
| `0x09` | 9 | `SetGlobal` | `[name_index: u8]` | `[..., value] -> [..., value]` | Updates existing global variable. |
| `0x0A` | 10 | `GetUpvalue` | `[upvalue_index: u8]` | `[...] -> [..., value]` | Reads captured upvalue from closure environment. |
| `0x0B` | 11 | `SetUpvalue` | `[upvalue_index: u8]` | `[..., value] -> [..., value]` | Writes to captured upvalue in closure environment. |
| `0x0C` | 12 | `GetProperty` | `[name_index: u8]` | `[..., instance] -> [..., value \| method]` | Accesses an object field or binds a method. |
| `0x0D` | 13 | `SetProperty` | `[name_index: u8]` | `[..., instance, value] -> [..., value]` | Sets a field on an instance. |
| `0x0E` | 14 | `GetSuper` | `[name_index: u8]` | `[..., superclass] -> [..., bound_method]` | Resolves superclass method using current `this`. |
| `0x0F` | 15 | `Equal` | None | `[..., a, b] -> [..., a == b]` | Compares equality between top two values. |
| `0x10` | 16 | `Greater` | None | `[..., a, b] -> [..., a > b]` | Floating point greater-than comparison. |
| `0x11` | 17 | `Less` | None | `[..., a, b] -> [..., a < b]` | Floating point less-than comparison. |
| `0x12` | 18 | `Add` | None | `[..., a, b] -> [..., a + b]` | Numeric addition or string concatenation. |
| `0x13` | 19 | `Subtract` | None | `[..., a, b] -> [..., a - b]` | Floating point subtraction (`a - b`). |
| `0x14` | 20 | `Multiply` | None | `[..., a, b] -> [..., a * b]` | Floating point multiplication (`a * b`). |
| `0x15` | 21 | `Divide` | None | `[..., a, b] -> [..., a / b]` | Floating point division (`a / b`). |
| `0x16` | 22 | `Not` | None | `[..., value] -> [..., !truthy(value)]` | Logical NOT (`nil` and `false` are falsy). |
| `0x17` | 23 | `Negate` | None | `[..., number] -> [..., -number]` | Unary arithmetic negation. |
| `0x18` | 24 | `Print` | None | `[..., value] -> [...]` | Pops value and writes to standard output + newline. |
| `0x19` | 25 | `Jump` | `[hi: u8, lo: u8]` | `[...] -> [...]` | Unconditional forward jump by 16-bit offset. |
| `0x1A` | 26 | `JumpIfFalse` | `[hi: u8, lo: u8]` | `[..., cond] -> [..., cond]` | Jumps forward if condition is falsy (leaves cond). |
| `0x1B` | 27 | `Loop` | `[hi: u8, lo: u8]` | `[...] -> [...]` | Unconditional backward jump by 16-bit offset. |
| `0x1C` | 28 | `Call` | `[arg_count: u8]` | `[..., callee, arg1..N] -> [..., return_val]` | Invokes function/closure/class constructor. |
| `0x1D` | 29 | `Invoke` | `[name_idx: u8, argc: u8]` | `[..., receiver, arg1..N] -> [..., return_val]` | Fused method lookup + invocation optimization. |
| `0x1E` | 30 | `SuperInvoke` | `[name_idx: u8, argc: u8]` | `[..., receiver, arg1..N] -> [..., return_val]` | Fused superclass method invocation. |
| `0x1F` | 31 | `Closure` | `[fn_idx: u8, upvalues...]`| `[...] -> [..., closure]` | Instantiates a closure capturing upvalue pointers. |
| `0x20` | 32 | `CloseUpvalue` | None | `[..., value] -> [...]` | Closes open upvalue on stack and pops value. |
| `0x21` | 33 | `Return` | None | `[..., result] -> [caller_stack..., result]` | Returns from current frame to caller. |
| `0x22` | 34 | `Class` | `[name_index: u8]` | `[...] -> [..., class_obj]` | Creates a new class descriptor. |
| `0x23` | 35 | `Inherit` | None | `[..., super, sub] -> [..., super, sub]` | Copies methods from superclass to subclass. |
| `0x24` | 36 | `Method` | `[name_index: u8]` | `[..., class, closure] -> [..., class]` | Binds closure as a method on the target class. |
| `0x25` | 37 | `BwAnd` | None | `[..., a, b] -> [..., a & b]` | Bitwise AND (operands converted to integer). |
| `0x26` | 38 | `BwOr` | None | `[..., a, b] -> [..., a \| b]` | Bitwise OR (operands converted to integer). |
| `0x27` | 39 | `BwXor` | None | `[..., a, b] -> [..., a ^ b]` | Bitwise XOR (operands converted to integer). |
| `0x28` | 40 | `BwNot` | None | `[..., value] -> [..., ~value]` | Bitwise NOT (operand converted to integer). |
| `0x29` | 41 | `ShiftLeft` | None | `[..., a, b] -> [..., a << b]` | Bitwise shift left (`a << b`). |
| `0x2A` | 42 | `ShiftRight` | None | `[..., a, b] -> [..., a >> b]` | Bitwise shift right (`a >> b`). |
| `0x2B` | 43 | `ArrayBuild` | `[count: u8]` | `[..., item1..N] -> [..., array_obj]` | Creates an array containing the top `count` values. |
| `0x2C` | 44 | `ArrayIndex` | None | `[..., array, index] -> [..., element]` | Subscript index read (`array[index]`). |
| `0x2D` | 45 | `ArrayStore` | None | `[..., array, index, val] -> [..., val]` | Subscript index write (`array[index] = val`). |
| `0x2E` | 46 | `Import` | `[const_idx: u8]` | `[] -> []` | Dynamically loads and binds a module (built-in or foreign `.so`) into VM globals. |

---

## 3. Detailed Semantic Mechanics

### 3.1 Closures and Upvalues (`Opcode::Closure`)
The `Closure` instruction has dynamic operands depending on how many upvalues the target function captures:
```text
[Opcode::Closure] [func_constant_index: u8]
    [is_local: u8] [index: u8]   // Upvalue 0
    [is_local: u8] [index: u8]   // Upvalue 1
    ...
```
* `is_local == 1`: The upvalue captures a local variable from the immediately enclosing scope slot `index`.
* `is_local == 0`: The upvalue captures an already-existing upvalue at `index` from the enclosing closure's upvalue array.

### 3.2 Jumps & Control Flow
Jump instructions (`Jump`, `JumpIfFalse`, `Loop`) encode offsets using two consecutive bytes:
$$\text{offset} = (\text{operand\_hi} \ll 8) \mid \text{operand\_lo}$$
* `Jump` and `JumpIfFalse` advance the instruction pointer: $\text{ip} \leftarrow \text{ip} + \text{offset}$.
* `Loop` moves the instruction pointer backwards: $\text{ip} \leftarrow \text{ip} - \text{offset}$.
