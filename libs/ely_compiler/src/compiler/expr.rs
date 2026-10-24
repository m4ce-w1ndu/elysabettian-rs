use crate::compiler::Compiler;
use crate::parser::Precedence;
use crate::token::{Token, TokenType};
use ely_core::opcode::Opcode;
use ely_core::value::Value;

impl<'a> Compiler<'a> {
    /// Compiles an expression at the assignment precedence level.
    pub(crate) fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    /// Compiles a numeric constant literal.
    pub(crate) fn number(&mut self) {
        let value: f64 = self.parser.previous.lexeme.parse().unwrap_or(0.0);
        let value = Value::number(value);
        self.emit_constant(value);
    }

    /// Parses an expression that starts with a grouping open parenthesis.
    pub(crate) fn grouping(&mut self) {
        self.expression();
        self.parser
            .consume(TokenType::CloseParen, "Expected ')' after expression.");
    }

    /// Parses an expression starting with a unary prefix operator.
    pub(crate) fn unary(&mut self) {
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
    pub(crate) fn binary(&mut self) {
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
    pub(crate) fn literal(&mut self) {
        match self.parser.previous.token_type {
            TokenType::True => self.emit_opcode(Opcode::True),
            TokenType::False => self.emit_opcode(Opcode::False),
            TokenType::Null => self.emit_opcode(Opcode::Nop),
            _ => (),
        }
    }

    /// Parses a string literal, interns it into the string pool, and emits a constant object reference.
    pub(crate) fn string(&mut self) {
        let lexeme = self.parser.previous.lexeme;
        let value = &lexeme[1..lexeme.len() - 1];
        let string_idx = self.chunk.add_string_constant(value.to_string());

        self.emit_constant(Value::obj(string_idx as u64));
    }

    /// Emits bytecode to resolve or assign a named identifier.
    pub(crate) fn named_variable(&mut self, name: Token<'a>, can_assign: bool) {
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

    /// Parses a variable reference or assignment.
    pub(crate) fn variable(&mut self, can_assign: bool) {
        self.named_variable(self.parser.previous, can_assign);
    }

    /// Compiles logical `and` with short-circuit forward jump.
    pub(crate) fn and(&mut self) {
        let end_jump = self.emit_jump(Opcode::JumpIfFalse);
        self.emit_opcode(Opcode::Pop);
        self.parse_precedence(Precedence::And);
        self.patch_jump(end_jump);
    }

    /// Compiles logical `or` with short-circuit forward jump.
    pub(crate) fn or(&mut self) {
        let else_jump = self.emit_jump(Opcode::JumpIfFalse);
        let end_jump = self.emit_jump(Opcode::Jump);
        self.patch_jump(else_jump);
        self.emit_opcode(Opcode::Pop);
        self.parse_precedence(Precedence::Or);
        self.patch_jump(end_jump);
    }

    /// Parses comma-separated arguments in a function call, returning the argument count.
    pub(crate) fn argument_list(&mut self) -> u8 {
        let mut arg_count: u8 = 0;

        if !self.parser.check(TokenType::CloseParen) {
            loop {
                self.expression();

                if arg_count == 255 {
                    self.parser.error("Can't have more than 255 arguments.");
                }

                arg_count += 1;

                if !self.parser.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        self.parser
            .consume(TokenType::CloseParen, "Expected ')' after arguments.");
        arg_count
    }

    /// Parses a function invocation expression.
    pub(crate) fn call(&mut self) {
        let arg_count = self.argument_list();
        self.emit_opcode(Opcode::Call);
        self.emit_byte(arg_count);
    }

    /// Parses an expression with precedence equal to or greater than the specified level.
    pub(crate) fn parse_precedence(&mut self, precedence: Precedence) {
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
            match self.parser.previous.token_type {
                TokenType::And => self.and(),
                TokenType::Or => self.or(),
                TokenType::OpenParen => self.call(),
                _ => self.binary(),
            }
        }

        if can_assign && self.parser.match_token(TokenType::Assign) {
            self.parser.error("Invalid assignment target.");
        }
    }

    /// Resolves the infix precedence level associated with a token type.
    pub(crate) fn get_precedence(token_type: TokenType) -> Precedence {
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
            TokenType::And => Precedence::And,
            TokenType::Or => Precedence::Or,
            TokenType::OpenParen => Precedence::Call,
            _ => Precedence::None,
        }
    }
}
