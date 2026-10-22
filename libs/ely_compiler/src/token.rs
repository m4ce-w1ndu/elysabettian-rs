/// Represents the classification of lexical tokens recognized by the Elysabettian tokenizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    OpenParen,
    CloseParen,
    OpenCurly,
    CloseCurly,
    OpenSquare,
    CloseSquare,

    Comma,
    Dot,
    Semicolon,

    Minus,
    Plus,
    Divide,
    Multiply,
    BwNot,
    BwXor,

    Negate,
    NonEquality,
    Assign,
    Equality,

    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,

    BwAnd,
    And,
    BwOr,
    Or,
    ShiftLeft,
    ShiftRight,

    Identifier,
    String,
    Number,

    If,
    Else,
    While,
    For,
    Return,
    Var,
    Func,
    Class,
    True,
    False,
    Null,
    This,
    Super,
    Import,

    Print,

    Error,
    Eof,
}

/// Represents a lexical token emitted by the scanner, referencing a slice of the source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token<'a> {
    /// Category of this token.
    pub token_type: TokenType,
    /// Exact textual slice in the original source code.
    pub lexeme: &'a str,
    /// 1-based source line where the token was encountered.
    pub line: u32,
}

impl<'a> Token<'a> {
    /// Constructs a new lexical token.
    #[inline]
    pub fn new(token_type: TokenType, lexeme: &'a str, line: u32) -> Self {
        Self {
            token_type,
            lexeme,
            line,
        }
    }
}
