use crate::scanner::Scanner;
use crate::token::{Token, TokenType};
use ely_core::chunk::Chunk;
use ely_core::interner::StringInterner;
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
    interner: &'a mut StringInterner,
    scope_depth: u32,
}

impl<'a> Compiler<'a> {
    /// Creates a new compiler attached to the given parser.
    pub fn new(parser: Parser<'a>, interner: &'a mut StringInterner) -> Self {
        Self {
            parser,
            chunk: Chunk::new(),
            interner,
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

    fn expression(&mut self) {
        todo!();
    }

    fn number(&mut self) {
        let value: f64 = self.parser.previous.lexeme.parse().unwrap_or(0.0);
        let value = Value::number(value);
        self.emit_constant(value);
    }

    /// Parses an expression that starts with a grouping open parenthesis.
    fn grouping(&mut self) {
        self.expression();
        self.parser
            .consume(TokenType::CloseParen, "Expected ')' after expression.");
    }

    /// Parses an expression starting with a unary prefix operator.
    fn unary(&mut self) {
        let operator_type = self.parser.previous.token_type;
        self.parse_precedence(Precedence::Unary);

        match operator_type {
            TokenType::Minus => self.emit_opcode(Opcode::Negate),
            TokenType::Negate => self.emit_opcode(Opcode::Not),
            TokenType::BwNot => self.emit_opcode(Opcode::BwNot),
            _ => (),
        }
    }

    /// Parses an infix binary expression and emits the corresponding opcode.
    fn binary(&mut self) {
        let operator_type = self.parser.previous.token_type;
        let precedence = Self::get_precedence(operator_type);
        self.parse_precedence(precedence.next());

        match operator_type {
            TokenType::Plus => self.emit_opcode(Opcode::Add),
            TokenType::Minus => self.emit_opcode(Opcode::Subtract),
            TokenType::Multiply => self.emit_opcode(Opcode::Multiply),
            TokenType::Divide => self.emit_opcode(Opcode::Divide),
            TokenType::Equality => self.emit_opcode(Opcode::Equal),
            TokenType::NonEquality => {
                self.emit_opcode(Opcode::Equal);
                self.emit_opcode(Opcode::Not);
            }
            TokenType::GreaterThan => self.emit_opcode(Opcode::Greater),
            TokenType::LessThan => self.emit_opcode(Opcode::Less),
            TokenType::GreaterThanEqual => {
                self.emit_opcode(Opcode::Less);
                self.emit_opcode(Opcode::Not);
            }
            TokenType::LessThanEqual => {
                self.emit_opcode(Opcode::Greater);
                self.emit_opcode(Opcode::Not);
            }
            TokenType::BwAnd => self.emit_opcode(Opcode::BwAnd),
            TokenType::BwOr => self.emit_opcode(Opcode::BwOr),
            TokenType::BwXor => self.emit_opcode(Opcode::BwXor),
            TokenType::ShiftLeft => self.emit_opcode(Opcode::ShiftLeft),
            TokenType::ShiftRight => self.emit_opcode(Opcode::ShiftRight),
            _ => (),
        }
    }

    /// Parses a literal keyword expression (`true`, `false`, `null`).
    fn literal(&mut self) {
        match self.parser.previous.token_type {
            TokenType::True => self.emit_opcode(Opcode::True),
            TokenType::False => self.emit_opcode(Opcode::False),
            TokenType::Null => self.emit_opcode(Opcode::Nop),
            _ => (),
        }
    }

    /// Parses a string literal, interns it into the string pool, and emits a constant object reference.
    fn string(&mut self) {
        let lexeme = self.parser.previous.lexeme;
        let value = &lexeme[1..lexeme.len() - 1];
        let string_id = self.interner.intern(value);

        self.emit_constant(Value::obj(string_id as u64));
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        todo!();
    }

    /// Resolves the infix precedence level associated with a token type.
    fn get_precedence(token_type: TokenType) -> Precedence {
        match token_type {
            TokenType::Multiply | TokenType::Divide => Precedence::Factor,
            TokenType::Plus | TokenType::Minus => Precedence::Term,
            TokenType::LessThan
            | TokenType::LessThanEqual
            | TokenType::GreaterThan
            | TokenType::GreaterThanEqual => Precedence::Comparison,
            TokenType::Equality | TokenType::NonEquality => Precedence::Equality,
            TokenType::BwOr
            | TokenType::BwXor
            | TokenType::BwAnd
            | TokenType::ShiftLeft
            | TokenType::ShiftRight => Precedence::Term,
            _ => Precedence::None,
        }
    }
}
