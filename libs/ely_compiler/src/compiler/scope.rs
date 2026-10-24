use crate::compiler::{Compiler, LocalVar};
use crate::token::{Token, TokenType};
use ely_core::opcode::Opcode;
use ely_core::value::Value;

impl<'a> Compiler<'a> {
    /// Increments the current lexical block scope depth.
    pub(crate) fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    /// Exits the current scope, popping all locally declared variables from the runtime stack.
    pub(crate) fn end_scope(&mut self) {
        self.scope_depth -= 1;

        while self.locals.last().is_some()
            && self.locals.last().unwrap().depth > self.scope_depth as i32
        {
            self.locals.pop();
            self.emit_opcode(Opcode::Pop);
        }
    }

    /// Registers a new local variable in the compiler's tracking table.
    pub(crate) fn add_local(&mut self, name: Token<'a>) {
        if self.locals.len() >= 256 {
            self.parser.error("Too many local variables in function.");
            return;
        }

        self.locals.push(LocalVar {
            name,
            depth: -1,
            is_capture: false,
        });
    }

    /// Declares a local variable in the current block scope, rejecting duplicates in the same depth.
    pub(crate) fn declare_variable(&mut self) {
        if self.scope_depth == 0 {
            return;
        }

        let name = self.parser.previous;

        for var in self.locals.iter().rev() {
            if var.depth != -1 && var.depth < self.scope_depth as i32 {
                break;
            }

            if name.lexeme == var.name.lexeme {
                self.parser
                    .error("Already a variable with this name in this scope.");
                return;
            }
        }

        self.add_local(name);
    }

    /// Marks the most recently added local variable as fully initialized and ready for use.
    pub(crate) fn mark_initialized(&mut self) {
        if self.scope_depth == 0 {
            return;
        }

        if let Some(var) = self.locals.last_mut() {
            var.depth = self.scope_depth as i32;
        }
    }

    /// Resolves a local variable by identifier name, returning its stack slot if found.
    pub(crate) fn resolve_local(&mut self, name: Token<'a>) -> Option<u8> {
        for (i, var) in self.locals.iter().enumerate().rev() {
            if name.lexeme == var.name.lexeme {
                if var.depth == -1 {
                    self.parser
                        .error("Can't read local variable in its own initializer.");
                    return None;
                }

                return Some(i as u8);
            }
        }

        None
    }

    /// Interns an identifier token and emits it as a constant, returning the constant index.
    pub(crate) fn identifier_constant(&mut self, token: Token<'a>) -> u8 {
        let name_id = self.interner.intern(token.lexeme);
        let constant_val = Value::obj(name_id as u64);
        let idx = self.chunk.add_constant(constant_val);

        if idx > u8::MAX as usize {
            self.parser.error("Too many constants in one chunk.");
            return 0;
        }

        idx as u8
    }

    /// Consumes an identifier token and registers it in the constant pool.
    pub(crate) fn parse_variable(&mut self, error_msg: &str) -> u8 {
        self.parser.consume(TokenType::Identifier, error_msg);
        self.declare_variable();
        if self.scope_depth > 0 {
            return 0;
        }

        self.identifier_constant(self.parser.previous)
    }

    /// Emits global definition bytecode or marks the local variable as initialized.
    pub(crate) fn define_variable(&mut self, global_constant_idx: u8) {
        if self.scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        if self.scope_depth == 0 {
            self.emit_opcode(Opcode::DefineGlobal);
            self.emit_byte(global_constant_idx);
        }
    }
}
