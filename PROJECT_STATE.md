# Elysabettian-rs: Context & Architecture State

## Obiettivo del Progetto
Porting completo della virtual machine ad alte prestazioni e del compilatore bytecode Pratt da C++ (`/home/quark/Projects/elysabettian`) a un'architettura moderna in Rust (`/home/quark/Projects/elysabettian-rs`), culminante in un motore JIT con Cranelift.

---

## Struttura del Workspace
- **`libs/ely_core`**: 
  - 46 Opcodes (`Opcode` enum con mapping 1:1 con il runtime C++).
  - NaN-Boxing 64-bit IEEE-754 (`Value`).
  - Bytecode chunk (`Chunk`), costanti e linea compatta (`LineInfo`).
  - Disassembler e interner.
- **`libs/ely_compiler`**: 
  - Pratt Parser per espressioni e precedenze.
  - Scanner con gestione token e literal.
  - Statement (`print`, `expression`, `block`).
  - Risoluzione variabili locali e globali, scope annidati.
  - Control flow: jumps, loop, if/else, while, for, corto-circuito `&&`/`||`.
  - Funzioni: parametri, arity, closures.
  - 9 test unitari passando al 100%.
- **`libs/ely_vm`**:
  - VM pura in Safe Rust con `Rc<Chunk>` (nessun puntatore grezzo / unsafe).
  - Multi-frame call stack (`CallFrame`) con limite `MAX_FRAMES = 64`.
  - Strutture heap (`HeapObject`): `String`, `Array`, `Function`, `Closure`, `Upvalue`, `Class`, `Instance`, `BoundMethod`.
  - Aritmetica, confronti, operatori bitwise completi (`BwAnd`, `BwOr`, `BwXor`, `BwNot`, `ShiftLeft`, `ShiftRight`).
  - Chiusure lessicali e upvalue aperti/chiusi (`capture_upvalue`, `close_upvalue`).
  - OOP completo: `Class`, `Method`, `GetProperty`, `SetProperty`, `Inherit`, `GetSuper`, `Invoke`, `SuperInvoke`.
  - 6 unit test passando al 100%.
- **`libs/ely_jit`**:
  - `JitEngine`: Target ISA detection, memory management con Cranelift JITModule.
  - `JitCompiler`: Lowering del bytecode in Cranelift SSA IR.
  - Supporto ad aritmetica floating-point 64-bit hardware (`fadd`, `fsub`, `fmul`, `fdiv`, `fneg`).
  - Supporto a parametri ed allocazione registri SSA per variabili locali (`GetLocal`, `SetLocal`).
  - Control Flow Graph completo con blocchi Cranelift per salti in avanti e all'indietro (`Jump`, `JumpIfFalse`, `Loop`).
  - Confronti in virgola mobile (`Less`, `Greater`, `Equal`).
  - Unit test JIT passando al 100% (inclusa esecuzione nativa di loop e accumulatore).
- **`tools/ely_cli`**:

  - Entry point per CLI REPL e runner script `.ely`.

---

## Prossimi Passi
1. **Integrazione VM <-> JIT**: Aggiungere un tier-up / hot-path detection nella VM (o annotazione / esecuzione diretta) per dirottare l'esecuzione di funzioni e loop intensivi dal loop di dispatch della VM al codice macchina nativo JIT compilato.


