use crate::opcode::Opcode;
use crate::value::Value;

/// Represents a source line mapping checkpoint in compiled bytecode.
///
/// Stores the starting bytecode offset where a particular source line begins,
/// enabling compact line metadata storage and fast logarithmic lookup via binary search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineInfo {
    /// Bytecode offset where instructions for this line start.
    pub offset: u32,
    /// Source line number corresponding to this code offset.
    pub line: u32,
}

/// A contiguous chunk of compiled bytecode, constant pool values, and debug line mappings.
///
/// `Chunk` acts as the primary compilation unit emitted by the frontend compiler
/// and directly consumed by the virtual machine interpreter or lowered to Cranelift IR.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Chunk {
    /// Raw bytecode instruction stream and inline operands.
    pub code: Vec<u8>,
    /// Constant values referenced by bytecode instructions via indexed lookup.
    pub constants: Vec<Value>,
    /// Compact line metadata table sorted by bytecode offset.
    pub lines: Vec<LineInfo>,
}

impl Chunk {
    /// Constructs a new, empty Chunk.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a raw byte to the instruction stream and records its source line mapping.
    ///
    /// To optimize memory usage, a new `LineInfo` checkpoint is only recorded
    /// when the source line differs from the preceding instruction.
    pub fn write_byte(&mut self, byte: u8, line: u32) {
        let offset = self.code.len() as u32;
        self.code.push(byte);

        if let Some(last) = self.lines.last() {
            if last.line == line {
                return;
            }
        }

        self.lines.push(LineInfo { offset, line })
    }

    /// Appends a typed opcode instruction to the bytecode stream.
    #[inline]
    pub fn write_opcode(&mut self, opcode: Opcode, line: u32) {
        let byte = opcode as u8;
        self.write_byte(byte, line);
    }

    /// Appends a value to the chunk's constant pool and returns its index.
    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    /// Retrieves a value from the constant pool by its index.
    #[inline]
    pub fn get_constant(&self, index: usize) -> Option<Value> {
        self.constants.get(index).copied()
    }
}
