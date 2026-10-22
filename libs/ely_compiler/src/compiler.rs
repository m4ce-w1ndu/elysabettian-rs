use crate::scanner::Scanner;
use crate::token::{Token, TokenType};
use ely_core::chunk::Chunk;
use ely_core::opcode::Opcode;
use ely_core::value::Value;

/// Expression precedence levels ordered from lowest to highest for Pratt parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Precedence {
    None = 0,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

impl Precedence {
    /// Returns the immediate higher precedence level.
    #[inline]
    pub fn next(self) -> Self {
        match self {
            Precedence::None => Precedence::Assignment,
            Precedence::Assignment => Precedence::Or,
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => Precedence::Primary,
        }
    }
}

/// Maintains the lexical scanner and syntax error state during compilation.
pub struct Parser<'a> {
    scanner: Scanner<'a>,
    current: Token<'a>,
    previous: Token<'a>,
    had_error: bool,
    panic_mode: bool,
}

impl<'a> Parser<'a> {
    /// Initializes a new parser with an initial EOF placeholder token.
    pub fn new(scanner: Scanner<'a>) -> Self {
        Self {
            scanner,
            current: Token::new(TokenType::Eof, "", 0),
            previous: Token::new(TokenType::Eof, "", 0),
            had_error: false,
            panic_mode: false,
        }
    }

    /// Emits a syntax error at the given token location unless currently in panic mode.
    pub fn error_at(&mut self, token: Token<'a>, message: &str) {
        if self.panic_mode {
            return;
        }

        self.panic_mode = true;
        self.had_error = true;

        eprint!("[line {}] Error", token.line);

        if token.token_type == TokenType::Eof {
            eprint!(" at end");
        } else if token.token_type != TokenType::Error {
            eprint!(" at '{}'", token.lexeme);
        }

        eprintln!(": {}", message);
    }

    /// Emits a syntax error at the previous token position.
    pub fn error(&mut self, message: &str) {
        self.error_at(self.previous, message)
    }

    /// Emits a syntax error at the current token position.
    pub fn error_at_current(&mut self, message: &str) {
        self.error_at(self.current, message)
    }

    /// Advances the parser by consuming the current token and fetching the next valid token.
    pub fn advance(&mut self) {
        self.previous = self.current;

        loop {
            self.current = self.scanner.scan_token();

            if self.current.token_type != TokenType::Error {
                break;
            }

            self.error_at_current(self.current.lexeme);
        }
    }

    /// Consumes the current token if it matches `expected`, otherwise raises a syntax error.
    pub fn consume(&mut self, expected: TokenType, message: &str) {
        if self.current.token_type == expected {
            self.advance();
            return;
        }

        self.error_at_current(message);
    }

    /// Checks whether the current token matches the expected token type without consuming it.
    pub fn check(&self, expected: TokenType) -> bool {
        if self.current.token_type == TokenType::Eof {
            return false;
        }

        self.current.token_type == expected
    }

    /// Conditionally consumes the current token if it matches `expected`.
    pub fn match_token(&mut self, expected: TokenType) -> bool {
        if !self.check(expected) {
            return false;
        }

        self.advance();
        true
    }
}

/// Compiles Elysabettian source code into executable bytecode chunks.
pub struct Compiler<'a> {
    parser: Parser<'a>,
    chunk: Chunk,
    scope_depth: u32,
}

impl<'a> Compiler<'a> {
    /// Creates a new compiler attached to the given parser.
    pub fn new(parser: Parser<'a>) -> Self {
        Self {
            parser,
            chunk: Chunk::new(),
            scope_depth: 0,
        }
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
}
