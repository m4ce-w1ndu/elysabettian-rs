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

    pub(crate) fn prefix_inc_dec(&mut self) {
        let is_inc = self.parser.previous.token_type == TokenType::PlusPlus;
        self.parser.consume(
            TokenType::Identifier,
            "Expected variable name after '++' or '--'.",
        );
        let name = self.parser.previous;

        // Figure out if the variable is local or global once to avoid multiple scope lookups
        let (get_op, set_op, arg) = if let Some(slot) = self.resolve_local(name) {
            (Opcode::GetLocal, Opcode::SetLocal, slot)
        } else {
            (
                Opcode::GetGlobal,
                Opcode::SetGlobal,
                self.identifier_constant(name),
            )
        };

        self.emit_opcode(get_op);
        self.emit_byte(arg);

        self.emit_constant(Value::number(1.0));

        if is_inc {
            self.emit_opcode(Opcode::Add);
        } else {
            self.emit_opcode(Opcode::Subtract);
        }

        self.emit_opcode(set_op);
        self.emit_byte(arg);
    }

    pub(crate) fn postfix_inc_dec(&mut self, get_op: Opcode, set_op: Opcode, arg: u8) {
        let is_inc = self.parser.previous.token_type == TokenType::PlusPlus;

        // Postfix trick: push the current value twice.
        // The first copy stays on the stack as the expression result (old value),
        // while the second copy is modified and saved back into the variable.
        self.emit_opcode(get_op);
        self.emit_byte(arg);

        self.emit_opcode(get_op);
        self.emit_byte(arg);

        self.emit_constant(Value::number(1.0));
        if is_inc {
            self.emit_opcode(Opcode::Add);
        } else {
            self.emit_opcode(Opcode::Subtract);
        }

        // SetLocal/SetGlobal leaves the updated value on top of the stack.
        // Pop that updated value so only the original old value remains on the stack.
        self.emit_opcode(set_op);
        self.emit_byte(arg);
        self.emit_opcode(Opcode::Pop);
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
        let (get_op, set_op, arg) = if let Some(slot) = self.resolve_local(name) {
            (Opcode::GetLocal, Opcode::SetLocal, slot)
        } else {
            (
                Opcode::GetGlobal,
                Opcode::SetGlobal,
                self.identifier_constant(name),
            )
        };

        if can_assign && self.parser.match_token(TokenType::Assign) {
            self.expression();
            self.emit_opcode(set_op);
            self.emit_byte(arg);
        } else if can_assign
            && (self.parser.match_token(TokenType::PlusAssign)
                || self.parser.match_token(TokenType::MinusAssign)
                || self.parser.match_token(TokenType::MultiplyAssign)
                || self.parser.match_token(TokenType::DivideAssign))
        {
            // Save the operator
            let op = self.parser.previous.token_type;

            // Read value
            self.emit_opcode(get_op);
            self.emit_byte(arg);

            self.expression();

            // Emit opcode based on operator type
            match op {
                TokenType::PlusAssign => self.emit_opcode(Opcode::Add),
                TokenType::MinusAssign => self.emit_opcode(Opcode::Subtract),
                TokenType::MultiplyAssign => self.emit_opcode(Opcode::Multiply),
                TokenType::DivideAssign => self.emit_opcode(Opcode::Divide),
                _ => (),
            }

            // Store result
            self.emit_opcode(set_op);
            self.emit_byte(arg);
        } else if self.parser.match_token(TokenType::PlusPlus)
            || self.parser.match_token(TokenType::MinusMinus)
        {
            self.postfix_inc_dec(get_op, set_op, arg);
        } else {
            self.emit_opcode(get_op);
            self.emit_byte(arg);
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
            TokenType::PlusPlus | TokenType::MinusMinus => self.prefix_inc_dec(),
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

        if can_assign
            && (self.parser.match_token(TokenType::Assign)
                || self.parser.match_token(TokenType::PlusAssign)
                || self.parser.match_token(TokenType::MinusAssign)
                || self.parser.match_token(TokenType::MultiplyAssign)
                || self.parser.match_token(TokenType::DivideAssign))
        {
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
