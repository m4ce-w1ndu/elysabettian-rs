/// Bytecode instruction opcodes for the Elysabettian Virtual Machine.
///
/// Each instruction represents a discrete operation executed by the stack-based
/// runtime or compiled down to native machine code by the Cranelift JIT engine.
///
/// Opcodes are laid out sequentially with explicit `u8` discriminant values to ensure
/// binary parity with the original C++ reference runtime. Instructions cover stack
/// evaluation, scoped variable resolution, lexical closures and upvalues,
/// class-based object orientation, dynamic array indexing, and bitwise arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    // Stack manipulation
    Constant = 0,
    Nop,
    True,
    False,
    Pop,

    // Memory objects operations
    GetLocal,
    GetGlobal,
    DefineGlobal,
    SetLocal,
    SetGlobal,
    GetUpvalue,
    SetUpvalue,
    GetProperty,
    SetProperty,
    GetSuper,

    // Comparison
    Equal,
    Greater,
    Less,

    // Unary and binary operations
    Add,
    Subtract,
    Multiply,
    Divide,
    Not,
    Negate,

    // Control flow and basic output
    Print,
    Jump,
    JumpIfFalse,
    Loop,

    // Routine handling
    Call,
    Invoke,
    SuperInvoke,
    Closure,
    CloseUpvalue,
    Return,

    // OOP-related
    Class,
    Inherit,
    Method,

    // Bit operations
    BwAnd,
    BwOr,
    BwXor,
    BwNot,
    ShiftLeft,
    ShiftRight,

    // Arrays
    ArrayBuild,
    ArrayIndex,
    ArrayStore,

    // Import external modules
    Import,
}

/// Implements safe conversion of the Opcode enum straight from u8
impl TryFrom<u8> for Opcode {
    type Error = u8;

    #[inline]
    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        if byte <= Opcode::Import as u8 {
            Ok(unsafe { std::mem::transmute(byte) })
        } else {
            Err(byte)
        }
    }
}
