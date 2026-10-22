use crate::token::{Token, TokenType};

/// Zero-allocation lexical scanner that tokenizes Elysabettian source code.
pub struct Scanner<'a> {
    source: &'a str,
    start: usize,
    current: usize,
    line: u32,
}

impl<'a> Scanner<'a> {
    /// Creates a new scanner initialized at the start of the source string.
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            start: 0,
            current: 0,
            line: 1,
        }
    }

    /// Returns true if the scanner has reached the end of the source string.
    #[inline]
    pub fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    /// Consumes and returns the current character, advancing the scanner position.
    pub fn advance(&mut self) -> char {
        let index = self.current;
        self.current += 1;
        self.source.as_bytes()[index] as char
    }

    /// Looks at the current character without consuming it.
    #[inline]
    pub fn peek(&self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        self.source.as_bytes()[self.current] as char
    }

    /// Looks ahead at the character following the current position without consuming it.
    #[inline]
    pub fn peek_next(&self) -> char {
        if self.is_at_end() || self.current + 1 >= self.source.len() {
            return '\0';
        }
        self.source.as_bytes()[self.current + 1] as char
    }

    /// Conditionally consumes the current character if it matches the expected character.
    pub fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }

        if self.source.as_bytes()[self.current] as char != expected {
            return false;
        }

        self.current += 1;
        true
    }

    /// Emits a token corresponding to the current scanned lexeme slice.
    #[inline]
    pub fn make_token(&self, token_type: TokenType) -> Token<'a> {
        Token::new(
            token_type,
            &self.source[self.start..self.current],
            self.line,
        )
    }

    /// Emits an error token carrying a static error message string.
    #[inline]
    pub fn error_token(&self, message: &'static str) -> Token<'a> {
        Token::new(TokenType::Error, message, self.line)
    }
}
