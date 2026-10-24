use ely_core::chunk::Chunk;
use ely_core::value::Value;
use std::rc::Rc;

/// Represents an active call frame in the virtual machine call stack.
pub struct CallFrame {
    pub chunk: Rc<Chunk>,
    pub ip: usize,
    pub slots: usize,
    pub closure_id: Option<usize>,
}

impl CallFrame {
    /// Creates a new call frame pointing at the beginning of the chunk.
    pub fn new(chunk: Rc<Chunk>, slots: usize) -> Self {
        Self {
            chunk,
            ip: 0,
            slots,
            closure_id: None,
        }
    }

    /// Creates a new call frame associated with a closure environment.
    pub fn with_closure(chunk: Rc<Chunk>, slots: usize, closure_id: usize) -> Self {
        Self {
            chunk,
            ip: 0,
            slots,
            closure_id: Some(closure_id),
        }
    }

    /// Reads the next byte from the chunk instruction stream and advances the IP.
    pub fn read_byte(&mut self) -> u8 {
        let byte = self.chunk.code[self.ip];
        self.ip += 1;
        byte
    }

    /// Reads an unsigned 16-bit big-endian integer from the chunk instruction stream.
    pub fn read_short(&mut self) -> u16 {
        self.ip += 2;
        ((self.chunk.code[self.ip - 2] as u16) << 8) | (self.chunk.code[self.ip - 1] as u16)
    }

    /// Reads a constant index byte and returns the value from the chunk constant pool.
    pub fn read_constant(&mut self) -> Value {
        let index = self.read_byte() as usize;
        self.chunk.constants[index]
    }
}
