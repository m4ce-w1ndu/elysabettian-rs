use crate::compiler::{Compiler, FunctionType};
use crate::token::TokenType;
use ely_core::opcode::Opcode;

impl<'a> Compiler<'a> {
    /// Dispatches and compiles a single statement.
    pub(crate) fn statement(&mut self) {
        if self.parser.match_token(TokenType::Print) {
            self.print_statement();
        } else if self.parser.match_token(TokenType::If) {
            self.if_statement();
        } else if self.parser.match_token(TokenType::While) {
            self.while_statement();
        } else if self.parser.match_token(TokenType::For) {
            self.for_statement();
        } else if self.parser.match_token(TokenType::Return) {
            self.return_statement();
        } else if self.parser.match_token(TokenType::OpenCurly) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
        }
    }

    /// Compiles a `print <expr>;` statement.
    pub(crate) fn print_statement(&mut self) {
        self.expression();
        self.parser
            .consume(TokenType::Semicolon, "Expected ';' after value.");
        self.emit_opcode(Opcode::Print);
    }

    /// Compiles an expression evaluated for side effects (`<expr>;`), discarding its result with Pop.
    pub(crate) fn expression_statement(&mut self) {
        self.expression();
        self.parser
            .consume(TokenType::Semicolon, "Expected ';' after expression.");
        self.emit_opcode(Opcode::Pop);
    }

    /// Dispatches top-level declarations (variable bindings, functions, or fallthrough statements).
    pub(crate) fn declaration(&mut self) {
        if self.parser.match_token(TokenType::Var) {
            self.var_declaration();
        } else if self.parser.match_token(TokenType::Func) {
            self.func_declaration();
        } else if self.parser.match_token(TokenType::Import) {
            self.import_statement();
        } else {
            self.statement();
        }

        if self.parser.panic_mode {
            self.synchronize();
        }
    }

    /// Compiles an import statement, supporting both import("name"); and import "name";
    pub(crate) fn import_statement(&mut self) {
        // Parentheses are optional to support both function-like and statement-like import syntax
        let has_paren = self.parser.match_token(TokenType::OpenParen);
        self.parser.consume(
            TokenType::String,
            "Expected module name string after 'import'.",
        );

        // Strip quotes from the module name literal
        let lexeme = self.parser.previous.lexeme;
        let module_name = if lexeme.len() >= 2 {
            &lexeme[1..lexeme.len() - 1]
        } else {
            ""
        };

        let string_idx = self.chunk.add_string_constant(module_name.to_string());

        if has_paren {
            self.parser
                .consume(TokenType::CloseParen, "Expected ')' after module name.");
        }
        self.parser
            .consume(TokenType::Semicolon, "Expected ';' after import statement.");

        self.emit_opcode(Opcode::Import);
        self.emit_byte(string_idx as u8);
    }

    /// Parses a variable declaration statement (`var <name> [= <expr>];`).
    pub(crate) fn var_declaration(&mut self) {
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

    /// Recovers the parser state to the next statement boundary after a syntax error.
    pub(crate) fn synchronize(&mut self) {
        self.parser.panic_mode = false;

        while self.parser.current.token_type != TokenType::Eof {
            if self.parser.previous.token_type == TokenType::Semicolon {
                return;
            }

            match self.parser.current.token_type {
                TokenType::Class
                | TokenType::Func
                | TokenType::Var
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Print
                | TokenType::Return => return,
                _ => (),
            }

            self.parser.advance();
        }
    }

    /// Parses a sequence of declarations enclosed in `{ ... }`.
    pub(crate) fn block(&mut self) {
        while self.parser.current.token_type != TokenType::CloseCurly
            && self.parser.current.token_type != TokenType::Eof
        {
            self.declaration();
        }

        self.parser
            .consume(TokenType::CloseCurly, "Expected '}' after block.");
    }

    /// Compiles an `if (<cond>) <then> [else <else>]` statement.
    pub(crate) fn if_statement(&mut self) {
        self.parser
            .consume(TokenType::OpenParen, "Expected '(' after 'if'.");
        self.expression();
        self.parser
            .consume(TokenType::CloseParen, "Expected ')' after condition.");

        let then_jump = self.emit_jump(Opcode::JumpIfFalse);
        self.emit_opcode(Opcode::Pop);
        self.statement();

        let else_jump = self.emit_jump(Opcode::Jump);

        self.patch_jump(then_jump);
        self.emit_opcode(Opcode::Pop);

        if self.parser.match_token(TokenType::Else) {
            self.statement();
        }

        self.patch_jump(else_jump);
    }

    /// Compiles a `while (<cond>) <body>` loop.
    pub(crate) fn while_statement(&mut self) {
        let loop_start = self.chunk.count();

        self.parser
            .consume(TokenType::OpenParen, "Expected '(' after 'while'.");
        self.expression();
        self.parser
            .consume(TokenType::CloseParen, "Expected ')' after condition.");

        let exit_jump = self.emit_jump(Opcode::JumpIfFalse);
        self.emit_opcode(Opcode::Pop);

        self.statement();
        self.emit_loop(loop_start);

        self.patch_jump(exit_jump);
        self.emit_opcode(Opcode::Pop);
    }

    /// Compiles a `for ([init]; [cond]; [incr]) <body>` loop.
    pub(crate) fn for_statement(&mut self) {
        self.begin_scope();
        self.parser
            .consume(TokenType::OpenParen, "Expected '(' after 'for'.");

        if self.parser.match_token(TokenType::Semicolon) {
            ()
        } else if self.parser.match_token(TokenType::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }

        let mut loop_start = self.chunk.count();
        let mut exit_jump: Option<usize> = None;
        if !self.parser.match_token(TokenType::Semicolon) {
            self.expression();
            self.parser
                .consume(TokenType::Semicolon, "Expected ';' after loop condition.");
            exit_jump = Some(self.emit_jump(Opcode::JumpIfFalse));
            self.emit_opcode(Opcode::Pop);
        }

        if !self.parser.match_token(TokenType::CloseParen) {
            let body_jump = self.emit_jump(Opcode::Jump);
            let increment_start = self.chunk.count();
            self.expression();
            self.emit_opcode(Opcode::Pop);

            self.parser
                .consume(TokenType::CloseParen, "Expected ')' after 'for' clauses.");
            self.emit_loop(loop_start);
            loop_start = increment_start;
            self.patch_jump(body_jump);
        }

        self.statement();
        self.emit_loop(loop_start);

        if let Some(exit_jump) = exit_jump {
            self.patch_jump(exit_jump);
            self.emit_opcode(Opcode::Pop);
        }

        self.end_scope();
    }

    /// Compiles a function body, returning the number of compiled parameters.
    pub(crate) fn function(&mut self) -> usize {
        self.begin_scope();
        self.parser
            .consume(TokenType::OpenParen, "Expected '(' after function name.");

        let mut arity: usize = 0;
        if !self.parser.check(TokenType::CloseParen) {
            loop {
                arity += 1;
                if arity > 255 {
                    self.parser.error("Cannot have more than 255 parameters.");
                    return arity;
                }

                let constant = self.parse_variable("Expected parameter name.");
                self.define_variable(constant);

                if !self.parser.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        self.parser.consume(
            TokenType::CloseParen,
            "Expected ')' after function parameters.",
        );
        self.parser.consume(
            TokenType::OpenCurly,
            "Expected '{' after function signature.",
        );

        self.block();
        self.end_scope();
        self.emit_return();

        arity
    }

    /// Compiles a function declaration, parsing signature and body.
    pub(crate) fn func_declaration(&mut self) {
        let global = self.parse_variable("Expected function name for signature.");
        let func_name = self.parser.previous.lexeme.to_string();

        if self.scope_depth > 0 {
            self.mark_initialized();
        }

        let prev_chunk = std::mem::replace(&mut self.chunk, ely_core::chunk::Chunk::new());
        let prev_type = self.function_type;
        self.function_type = FunctionType::Function;
        let prev_locals = std::mem::take(&mut self.locals);
        let prev_depth = self.scope_depth;
        self.scope_depth = 0;

        self.locals.push(crate::compiler::LocalVar {
            name: crate::token::Token {
                token_type: TokenType::Identifier,
                lexeme: "",
                line: 0,
            },
            depth: 0,
            is_capture: false,
        });

        let arity = self.function();

        let func_chunk = std::mem::replace(&mut self.chunk, prev_chunk);
        self.locals = prev_locals;
        self.scope_depth = prev_depth;
        self.function_type = prev_type;

        let desc = ely_core::chunk::FunctionDescriptor {
            arity,
            chunk: func_chunk,
            name: Some(func_name),
        };

        let func_idx = self.chunk.add_function_constant(desc);
        let const_idx = self
            .chunk
            .add_constant(ely_core::value::Value::obj(func_idx as u64));

        self.emit_opcode(Opcode::Closure);
        self.emit_byte(const_idx as u8);

        self.define_variable(global);
    }

    /// Compiles a `return [expr];` statement.
    pub(crate) fn return_statement(&mut self) {
        if self.function_type == FunctionType::Script {
            self.parser.error("Cannot return from top-level code.");
        }

        if self.parser.match_token(TokenType::Semicolon) {
            self.emit_return();
        } else {
            self.expression();
            self.parser
                .consume(TokenType::Semicolon, "Expected ';' after return value.");
            self.emit_opcode(Opcode::Return);
        }
    }
}
