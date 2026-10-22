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
