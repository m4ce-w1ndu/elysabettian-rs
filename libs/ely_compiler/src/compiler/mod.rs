pub mod expr;
pub mod scope;
pub mod stmt;

#[cfg(test)]
mod tests;

use crate::parser::Parser;
use crate::scanner::Scanner;
use crate::token::{Token, TokenType};
use ely_core::chunk::Chunk;
use ely_core::interner::StringInterner;
use ely_core::opcode::Opcode;
use ely_core::value::Value;

/// Represents a local variable tracked on the virtual machine stack frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalVar<'a> {
    pub name: Token<'a>,
    pub depth: i32,
    pub is_capture: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionType {
    Script,
    Function,
}

/// Compiles Elysabettian source code into executable bytecode chunks.
pub struct Compiler<'a> {
    pub(crate) parser: Parser<'a>,
    pub(crate) chunk: Chunk,
    pub(crate) interner: &'a mut StringInterner,
    pub(crate) locals: Vec<LocalVar<'a>>,
    pub(crate) scope_depth: u32,
    pub function_type: FunctionType,
}

impl<'a> Compiler<'a> {
    /// Creates a new compiler attached to the given parser.
    pub fn new(parser: Parser<'a>, interner: &'a mut StringInterner) -> Self {
        let mut compiler = Self {
            parser,
            chunk: Chunk::new(),
            interner,
            locals: Vec::new(),
            scope_depth: 0,
            function_type: FunctionType::Script,
        };

        compiler.locals.push(LocalVar {
            name: Token {
                token_type: TokenType::Identifier,
                lexeme: "",
                line: 0,
            },
            depth: 0,
            is_capture: false,
        });

        compiler
    }

    /// Returns the source line number of the most recently consumed token.
    #[inline]
    pub fn current_line(&self) -> u32 {
        self.parser.previous.line
    }

    /// Emits a single raw byte into the target chunk.
    #[inline]
    pub fn emit_byte(&mut self, byte: u8) {
        self.chunk.write_byte(byte, self.current_line());
    }

    /// Emits an opcode instruction into the target chunk.
    #[inline]
    pub fn emit_opcode(&mut self, opcode: Opcode) {
        self.chunk.write_opcode(opcode, self.current_line());
    }

    /// Emits two consecutive bytes into the target chunk.
    #[inline]
    pub fn emit_bytes(&mut self, byte1: u8, byte2: u8) {
        self.emit_byte(byte1);
        self.emit_byte(byte2);
    }

    /// Emits a standard return sequence (pushing null followed by Return).
    pub fn emit_return(&mut self) {
        self.emit_opcode(Opcode::Nop);
        self.emit_opcode(Opcode::Return);
    }

    /// Adds a value to the constant pool and emits an Opcode::Constant instruction.
    pub fn emit_constant(&mut self, value: Value) {
        let index = self.chunk.add_constant(value);

        if index > u8::MAX as usize {
            self.parser.error("Too many constants in one chunk.");
            return;
        }

        self.emit_opcode(Opcode::Constant);
        self.emit_byte(index as u8);
    }

    /// Emits a forward jump instruction with two placeholder operand bytes.
    pub fn emit_jump(&mut self, opcode: Opcode) -> usize {
        self.emit_opcode(opcode);
        self.emit_byte(0xff);
        self.emit_byte(0xff);
        self.chunk.count() - 2
    }

    /// Backpatches a previously emitted forward jump with the calculated offset.
    pub fn patch_jump(&mut self, offset: usize) {
        let jump = self.chunk.count() - offset - 2;

        if jump > u16::MAX as usize {
            self.parser.error("Too much code to jump over.");
        }

        self.chunk.code[offset] = ((jump >> 8) & 0xff) as u8;
        self.chunk.code[offset + 1] = (jump & 0xff) as u8;
    }

    /// Emits an unconditional backward jump instruction to `loop_start`.
    pub fn emit_loop(&mut self, loop_start: usize) {
        self.emit_opcode(Opcode::Loop);

        let offset = self.chunk.count() - loop_start + 2;
        if offset > u16::MAX as usize {
            self.parser.error("Loop body too large.");
        }

        self.emit_byte(((offset >> 8) & 0xff) as u8);
        self.emit_byte((offset & 0xff) as u8);
    }
}

/// Compiles an Elysabettian source code expression into a bytecode `Chunk`.
///
/// Returns `Some(Chunk)` on successful compilation, or `None` if syntax errors occurred.
pub fn compile(source: &str, interner: &mut StringInterner) -> Option<Chunk> {
    let scanner = Scanner::new(source);
    let mut parser = Parser::new(scanner);

    parser.advance();

    let mut compiler = Compiler::new(parser, interner);

    while !compiler.parser.match_token(TokenType::Eof) {
        compiler.declaration();
    }

    compiler.emit_return();

    if compiler.parser.had_error() {
        None
    } else {
        Some(compiler.chunk)
    }
}
