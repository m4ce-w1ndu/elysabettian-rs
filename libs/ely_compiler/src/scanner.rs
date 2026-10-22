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

    /// Skips whitespace characters and single-line comments (`//`), tracking newline increments.
    pub fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                ' ' | '\r' | '\t' => _ = self.advance(),
                '\n' => {
                    _ = self.advance();
                    self.line += 1;
                }
                '/' => {
                    if self.peek_next() != '/' {
                        return;
                    }

                    while self.peek() != '\n' && !self.is_at_end() {
                        _ = self.advance();
                    }
                }
                _ => return,
            }
        }
    }

    /// Scans and returns the next token from the source code.
    pub fn scan_token(&mut self) -> Token<'a> {
        self.skip_whitespace();
        self.start = self.current;

        if self.is_at_end() {
            return self.make_token(TokenType::Eof);
        }

        let c = self.advance();

        if c.is_ascii_alphabetic() || c == '_' {
            return self.identifier();
        }

        if c.is_ascii_digit() {
            return self.number();
        }

        match c {
            '(' => self.make_token(TokenType::OpenParen),
            ')' => self.make_token(TokenType::CloseParen),
            '{' => self.make_token(TokenType::OpenCurly),
            '}' => self.make_token(TokenType::CloseCurly),
            '[' => self.make_token(TokenType::OpenSquare),
            ']' => self.make_token(TokenType::CloseSquare),
            ';' => self.make_token(TokenType::Semicolon),
            ',' => self.make_token(TokenType::Comma),
            '.' => self.make_token(TokenType::Dot),
            '-' => self.make_token(TokenType::Minus),
            '+' => self.make_token(TokenType::Plus),
            '/' => self.make_token(TokenType::Divide),
            '*' => self.make_token(TokenType::Multiply),
            '~' => self.make_token(TokenType::BwNot),
            '^' => self.make_token(TokenType::BwXor),
            '!' => {
                let token_type = if self.match_char('=') {
                    TokenType::NonEquality
                } else {
                    TokenType::Negate
                };
                self.make_token(token_type)
            }
            '=' => {
                let token_type = if self.match_char('=') {
                    TokenType::Equality
                } else {
                    TokenType::Assign
                };
                self.make_token(token_type)
            }
            '<' => {
                let token_type = if self.match_char('=') {
                    TokenType::LessThanEqual
                } else if self.match_char('<') {
                    TokenType::ShiftLeft
                } else {
                    TokenType::LessThan
                };
                self.make_token(token_type)
            }
            '>' => {
                let token_type = if self.match_char('=') {
                    TokenType::GreaterThanEqual
                } else if self.match_char('>') {
                    TokenType::ShiftRight
                } else {
                    TokenType::GreaterThan
                };
                self.make_token(token_type)
            }
            '&' => {
                let token_type = if self.match_char('&') {
                    TokenType::And
                } else {
                    TokenType::BwAnd
                };
                self.make_token(token_type)
            }
            '|' => {
                let token_type = if self.match_char('|') {
                    TokenType::Or
                } else {
                    TokenType::BwOr
                };
                self.make_token(token_type)
            }
            '"' => self.string(),
            _ => self.error_token("Unexpected character."),
        }
    }

    /// Scans a double-quoted string literal, supporting multiline strings.
    fn string(&mut self) -> Token<'a> {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            _ = self.advance();
        }

        if self.is_at_end() {
            return self.error_token("Unterminated string.");
        }

        _ = self.advance();

        self.make_token(TokenType::String)
    }

    /// Scans a numeric literal (integer or floating-point with fractional part).
    fn number(&mut self) -> Token<'a> {
        while self.peek().is_ascii_digit() {
            _ = self.advance();
        }

        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            _ = self.advance();
            while self.peek().is_ascii_digit() {
                _ = self.advance();
            }
        }

        self.make_token(TokenType::Number)
    }

    /// Identifies whether the current identifier slice matches a reserved language keyword.
    fn identifier_type(&self) -> TokenType {
        match &self.source[self.start..self.current] {
            "class" => TokenType::Class,
            "else" => TokenType::Else,
            "false" => TokenType::False,
            "for" => TokenType::For,
            "func" => TokenType::Func,
            "if" => TokenType::If,
            "import" => TokenType::Import,
            "null" => TokenType::Null,
            "print" => TokenType::Print,
            "return" => TokenType::Return,
            "super" => TokenType::Super,
            "this" => TokenType::This,
            "true" => TokenType::True,
            "var" => TokenType::Var,
            "while" => TokenType::While,
            _ => TokenType::Identifier,
        }
    }

    /// Scans an identifier or keyword token.
    fn identifier(&mut self) -> Token<'a> {
        while self.peek().is_ascii_alphanumeric() || self.peek() == '_' {
            _ = self.advance();
        }

        let token_type = self.identifier_type();
        self.make_token(token_type)
    }
}
