use crate::scanner::Scanner;
use crate::token::{Token, TokenType};

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
    pub(crate) scanner: Scanner<'a>,
    pub(crate) current: Token<'a>,
    pub(crate) previous: Token<'a>,
    pub(crate) had_error: bool,
    pub(crate) panic_mode: bool,
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
        self.error_at(self.previous, message);
    }

    /// Emits a syntax error at the current lookahead token position.
    pub fn error_at_current(&mut self, message: &str) {
        self.error_at(self.current, message);
    }

    /// Advances the parser by consuming the next token from the scanner.
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

    /// Consumes the current token if it matches `expected`, otherwise emits a syntax error.
    pub fn consume(&mut self, expected: TokenType, message: &str) {
        if self.current.token_type == expected {
            self.advance();
            return;
        }

        self.error_at_current(message);
    }

    /// Checks whether the current token matches the given type without consuming it.
    pub fn check(&self, expected: TokenType) -> bool {
        self.current.token_type == expected
    }

    /// Consumes the current token if it matches `expected`, returning whether it matched.
    pub fn match_token(&mut self, expected: TokenType) -> bool {
        if !self.check(expected) {
            return false;
        }

        self.advance();
        true
    }

    /// Returns true if any syntax error was recorded.
    pub fn had_error(&self) -> bool {
        self.had_error
    }
}
