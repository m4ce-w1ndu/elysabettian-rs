use crate::scanner::Scanner;
use crate::token::{Token, TokenType};
use ely_core::chunk::Chunk;
use ely_core::interner::StringInterner;
use ely_core::opcode::Opcode::{self, SetGlobal};
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

/// Represents a local variable tracked on the virtual machine stack frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalVar<'a> {
    pub name: Token<'a>,
    pub depth: i32,
    pub is_capture: bool,
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
    locals: Vec<LocalVar<'a>>,
    scope_depth: u32,
}

impl<'a> Compiler<'a> {
    /// Creates a new compiler attached to the given parser.
    pub fn new(parser: Parser<'a>, interner: &'a mut StringInterner) -> Self {
        Self {
            parser,
            chunk: Chunk::new(),
            interner,
            locals: Vec::new(),
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
        self.parse_precedence(Precedence::Assignment);
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

    /// Parses an expression with precedence equal to or greater than the specified level.
    fn parse_precedence(&mut self, precedence: Precedence) {
        self.parser.advance();

        let can_assign = precedence <= Precedence::Assignment;
        match self.parser.previous.token_type {
            TokenType::OpenParen => self.grouping(),
            TokenType::Minus | TokenType::Negate | TokenType::BwNot => self.unary(),
            TokenType::Number => self.number(),
            TokenType::String => self.string(),
            TokenType::False | TokenType::True | TokenType::Null => self.literal(),
            TokenType::Identifier => self.variable(can_assign),
            _ => {
                self.parser.error("Expected expression.");
                return;
            }
        }

        while precedence <= Self::get_precedence(self.parser.current.token_type) {
            self.parser.advance();
            self.binary();
        }

        if can_assign && self.parser.match_token(TokenType::Assign) {
            self.parser.error("Invalid assignment target.");
        }
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

    /// Dispatches and compiles a single statement.
    fn statement(&mut self) {
        if self.parser.match_token(TokenType::Print) {
            self.print_statement();
        } else if self.parser.match_token(TokenType::OpenCurly) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else if self.parser.match_token(TokenType::Print) {
            self.print_statement();
        } else {
            self.expression_statement();
        }
    }

    /// Parses a `print <expr>;` statement and emits `Opcode::Print`.
    fn print_statement(&mut self) {
        self.expression();
        self.parser
            .consume(TokenType::Semicolon, "Expected ';' after value.");
        self.emit_opcode(Opcode::Print);
    }

    /// Parses an expression statement and emits `Opcode::Pop` to discard the evaluation result.
    fn expression_statement(&mut self) {
        self.expression();
        self.parser
            .consume(TokenType::Semicolon, "Expected ';' after expression.");
        self.emit_opcode(Opcode::Pop);
    }

    /// Parses a declaration or statement, synchronizing error state if panic mode was triggered.
    fn declaration(&mut self) {
        if self.parser.check(TokenType::Eof) {
            return;
        }

        if self.parser.match_token(TokenType::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }

        if self.parser.panic_mode {
            self.synchronize();
        }
    }

    /// Synchronizes the parser state after a syntax error by advancing to the next statement boundary.
    fn synchronize(&mut self) {
        self.parser.panic_mode = false;
        while !self.parser.check(TokenType::Eof) {
            if self.parser.previous.token_type == TokenType::Semicolon {
                return;
            }

            match self.parser.current.token_type {
                TokenType::Class
                | TokenType::Func
                | TokenType::Var
                | TokenType::For
                | TokenType::If
                | TokenType::Print
                | TokenType::Return => return,
                _ => (),
            }

            self.parser.advance();
        }
    }

    /// Interns an identifier token string and records it into the constant pool.
    fn identifier_constant(&mut self, token: Token<'a>) -> u8 {
        let string_id = self.interner.intern(token.lexeme);
        let const_idx = self.chunk.add_constant(Value::obj(string_id as u64));

        if const_idx > u8::MAX as usize {
            self.parser.error("Too many constants in one chunk.");
            0
        } else {
            const_idx as u8
        }
    }

    /// Consumes an identifier token and resolves its constant index for global binding.
    fn parse_variable(&mut self, error_msg: &str) -> u8 {
        self.parser.consume(TokenType::Identifier, error_msg);
        self.declare_variable();

        if self.scope_depth > 0 {
            return 0;
        }

        self.identifier_constant(self.parser.previous)
    }

    /// Emits global variable definition bytecode when in top-level scope.
    fn define_variable(&mut self, global_constant_idx: u8) {
        if self.scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        if self.scope_depth == 0 {
            self.emit_opcode(Opcode::DefineGlobal);
            self.emit_byte(global_constant_idx);
        }
    }

    /// Parses a variable declaration statement (`var <name> [= <expr>];`).
    fn var_declaration(&mut self) {
        let global = self.parse_variable("Expected variable name.");

        if self.parser.match_token(TokenType::Assign) {
            self.expression();
        } else {
            self.emit_opcode(Opcode::Nop);
        }

        self.parser.consume(
            TokenType::Semicolon,
            "Expected ';' after variable declaration.",
        );

        self.define_variable(global);
    }

    /// Emits bytecode to resolve or assign a named identifier.
    fn named_variable(&mut self, name: Token<'a>, can_assign: bool) {
        let arg = self.identifier_constant(name);

        if can_assign && self.parser.match_token(TokenType::Assign) {
            self.expression();
            if let Some(slot) = self.resolve_local(name) {
                self.emit_opcode(Opcode::SetLocal);
                self.emit_byte(slot);
            } else {
                self.emit_opcode(Opcode::SetGlobal);
                self.emit_byte(arg);
            }
        } else {
            if let Some(slot) = self.resolve_local(name) {
                self.emit_opcode(Opcode::GetLocal);
                self.emit_byte(slot);
            } else {
                self.emit_opcode(Opcode::GetGlobal);
                self.emit_byte(arg);
            }
        }
    }

    /// Parses a variable evaluation or assignment expression.
    fn variable(&mut self, can_assign: bool) {
        let name = self.parser.previous;
        self.named_variable(name, can_assign)
    }

    /// Enters a new block scope level.
    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    /// Exits the current block scope level, popping local variables off the stack.
    fn end_scope(&mut self) {
        self.scope_depth -= 1;

        while self.locals.last().is_some()
            && self.locals.last().unwrap().depth >= self.scope_depth as i32
        {
            self.locals.pop();
            self.emit_opcode(Opcode::Pop);
        }
    }

    /// Parses a sequence of declarations enclosed in `{ ... }`.
    fn block(&mut self) {
        while self.parser.current.token_type != TokenType::CloseCurly
            && self.parser.current.token_type != TokenType::Eof
        {
            self.declaration();
        }

        self.parser
            .consume(TokenType::CloseCurly, "Expected '}' after block.");
    }

    /// Adds a local variable entry to the tracking list with uninitialized depth (-1).
    fn add_local(&mut self, name: Token<'a>) {
        if self.locals.len() >= 256 {
            self.parser.error("Too many local variables in function.");
            return;
        }

        self.locals.push(LocalVar {
            name,
            depth: -1,
            is_capture: false,
        })
    }

    /// Declares a local variable in the current block scope, rejecting duplicates in the same depth.
    fn declare_variable(&mut self) {
        if self.scope_depth == 0 {
            return;
        }

        let name = self.parser.previous;

        for var in self.locals.iter().rev() {
            if var.depth != -1 && var.depth < self.scope_depth as i32 {
                break;
            }

            if var.name.lexeme == name.lexeme {
                self.parser
                    .error("Already a variable with this name in this scope.");
                return;
            }
        }

        self.add_local(name);
    }

    /// Marks the most recent local variable as fully initialized with the current scope depth.
    fn mark_initialized(&mut self) {
        if self.scope_depth <= 0 {
            return;
        }

        if self.locals.last().is_none() {
            return;
        }

        self.locals.last_mut().unwrap().depth = self.scope_depth as i32;
    }

    /// Resolves a local variable by scanning the stack backwards, returning its slot index.
    fn resolve_local(&mut self, name: Token<'a>) -> Option<u8> {
        if let Some(slot) = self
            .locals
            .iter()
            .rposition(|l| l.name.lexeme == name.lexeme)
        {
            if self.locals[slot].depth == -1 {
                self.parser
                    .error("Cannot read local variable in its own initializer.");
                return None;
            } else {
                return Some(slot as u8);
            }
        }

        None
    }

    /// Emits a 16-bit jump instruction with placeholder offset bytes for later back-patching.
    fn emit_jump(&mut self, opcode: Opcode) -> usize {
        self.emit_opcode(opcode);
        self.emit_byte(0xff);
        self.emit_byte(0xff);
        self.chunk.count() - 2
    }

    /// Back-patches a previously emitted 16-bit forward jump with the calculated offset.
    fn patch_jump(&mut self, offset: usize) {
        let jump = self.chunk.count() - offset - 2;
        if jump > u16::MAX as usize {
            self.parser.error("Too much code to jump over.");
            return;
        }

        self.chunk.code[offset] = ((jump >> 8) & 0xff) as u8;
        self.chunk.code[offset + 1] = (jump & 0xff) as u8;
    }

    /// Emits a 16-bit backward loop jump instruction targeting the given bytecode offset.
    fn emit_loop(&mut self, loop_start: usize) {
        self.emit_opcode(Opcode::Loop);
        let jump = self.chunk.count() - loop_start + 2;
        if jump > u16::MAX as usize {
            self.parser.error("Loop body too large.");
            return;
        }

        self.emit_byte(((jump >> 8) & 0xff) as u8);
        self.emit_byte((jump & 0xff) as u8);
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

    while !compiler.parser.check(TokenType::Eof) {
        compiler.declaration();
    }

    compiler.emit_return();

    if compiler.parser.had_error {
        None
    } else {
        Some(compiler.chunk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to decode only the sequential stream of opcodes from a compiled chunk.
    fn extract_opcodes(chunk: &Chunk) -> Vec<Opcode> {
        let mut opcodes = Vec::new();
        let mut offset = 0;
        while offset < chunk.count() {
            let op = Opcode::try_from(chunk.code[offset]).expect("Invalid opcode byte.");
            opcodes.push(op);

            offset += match op {
                Opcode::Constant
                | Opcode::DefineGlobal
                | Opcode::GetGlobal
                | Opcode::SetGlobal
                | Opcode::GetLocal
                | Opcode::SetLocal
                | Opcode::Call
                | Opcode::Class
                | Opcode::Method
                | Opcode::GetProperty
                | Opcode::SetProperty
                | Opcode::GetSuper
                | Opcode::ArrayBuild => 2,
                Opcode::Jump
                | Opcode::JumpIfFalse
                | Opcode::Loop
                | Opcode::Invoke
                | Opcode::SuperInvoke => 3,
                _ => 1,
            };
        }
        opcodes
    }

    #[test]
    fn test_compile_math_expression() {
        let mut interner = StringInterner::new();
        let chunk = compile("1 + 2 * 3;", &mut interner).expect("Compilation failed.");

        ely_core::debug::disassemble_chunk(&chunk, "test_math");

        let expected_opcodes = vec![
            Opcode::Constant, // 1
            Opcode::Constant, // 2
            Opcode::Constant, // 3
            Opcode::Multiply, // 2 * 3
            Opcode::Add,      // 1 + (2 * 3)
            Opcode::Pop,      // expression statement pop
            Opcode::Nop,      // implicit return null
            Opcode::Return,
        ];

        assert_eq!(extract_opcodes(&chunk), expected_opcodes);
        assert_eq!(chunk.constants.len(), 3);
        assert_eq!(chunk.constants[0].as_number(), 1.0);
        assert_eq!(chunk.constants[1].as_number(), 2.0);
        assert_eq!(chunk.constants[2].as_number(), 3.0);
    }

    #[test]
    fn test_compile_variables() {
        let mut interner = StringInterner::new();
        let source = "var x = 10; print x + 5; x = 20;";
        let chunk = compile(source, &mut interner).expect("Compilation failed.");

        ely_core::debug::disassemble_chunk(&chunk, "test_variables");

        // Check "x" variable interning
        let x_id = interner.intern("x");
        assert_eq!(interner.get(x_id), Some("x"));

        // List of expected opcodes
        let expected_opcodes = vec![
            Opcode::Constant,     // 10
            Opcode::DefineGlobal, // 'x'
            Opcode::GetGlobal,    // 'x'
            Opcode::Constant,     // 5
            Opcode::Add,          // +
            Opcode::Print,        // print
            Opcode::Constant,     // 20
            Opcode::SetGlobal,    // 'x'
            Opcode::Pop,          // expression statement
            Opcode::Nop,          // implicit return null
            Opcode::Return,       // return
        ];

        assert_eq!(extract_opcodes(&chunk), expected_opcodes);

        // Check numeric values
        assert!(
            chunk
                .constants
                .iter()
                .any(|c| c.is_number() && c.as_number() == 10.0)
        );
        assert!(
            chunk
                .constants
                .iter()
                .any(|c| c.is_number() && c.as_number() == 5.0)
        );
        assert!(
            chunk
                .constants
                .iter()
                .any(|c| c.is_number() && c.as_number() == 20.0)
        );
    }

    #[test]
    fn test_compile_local_variables_and_scopes() {
        let mut interner = StringInterner::new();
        let source = r#"
            var a = 1;
            {
                var b = 2;
                print a + b;
                b = 3;
            }
        "#;
        let chunk = compile(source, &mut interner).expect("Compilation failed.");

        ely_core::debug::disassemble_chunk(&chunk, "test_locals_scope");

        let expected_opcodes = vec![
            Opcode::Constant,     // a = 1
            Opcode::DefineGlobal, // 'a'
            Opcode::Constant,     // b = 2 (local slot 0)
            Opcode::GetGlobal,    // 'a'
            Opcode::GetLocal,     // slot 0 (b)
            Opcode::Add,
            Opcode::Print,
            Opcode::Constant, // 3
            Opcode::SetLocal, // slot 0 (b)
            Opcode::Pop,      // statement pop
            Opcode::Pop,      // end_scope pops local b off stack!
            Opcode::Nop,      // implicit return null
            Opcode::Return,
        ];

        assert_eq!(extract_opcodes(&chunk), expected_opcodes);
    }
}
