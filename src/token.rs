#[derive(Debug, Clone)]
pub struct Token<'a> {
    pub token_type: TokenType,
    pub lexeme: &'a str,
    pub literal: TokenLiteral<'a>,
    pub pos: TokenPosition,
}

impl <'a> Token<'a> {
    pub fn new(token_type: TokenType, lexeme: &'a str, literal: TokenLiteral<'a>, line: u64, offset: u64) -> Self {
        Self {
            token_type,
            lexeme: lexeme,
            literal,
            pos: TokenPosition { line, offset },
        }
    }

    pub fn symbol(token_type: TokenType, lexeme: &'a str, line: u64, offset: u64) -> Self {
        Self::new(token_type, lexeme.into(), TokenLiteral::NoValue, line, offset)
    }

    pub fn textual(value: &'a str, line: u64, offset: u64) -> Self {
        Self::new(identifier_type(&value), value, TokenLiteral::NoValue, line, offset)
    }

    pub fn string(value: &'a str, line: u64, offset: u64) -> Self {
        let value = &value[1..value.len() - 1];
        Self::new(TokenType::String, value, TokenLiteral::String(value), line, offset)
    }

    pub fn number(value: &'a str, line: u64, offset: u64) -> Self {
        let n = value.parse().unwrap();
        Self::new(TokenType::Number, value, TokenLiteral::Number(n), line, offset)
    }

    pub fn eof(line: u64) -> Self {
        Self::new(TokenType::Eof, "", TokenLiteral::NoValue, line, 0)
    }
}

#[derive(Debug, Clone)]
pub struct TokenPosition {
    pub line: u64,
    pub offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TokenType {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Plus,
    Minus,
    Dot,
    SemiColon,
    Star,
    Comma,
    Not,
    Asign,
    Equal,
    NotEqual,
    Less,
    LessEq,
    Greater,
    GreaterEq,
    String,
    Identifier,
    Number,
    Div,
    And,
    Class,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,
    Eof,
}

#[derive(Debug, PartialEq, Clone, PartialOrd)]
pub enum TokenLiteral<'a> {
    String(&'a str),
    Number(f64),
    NoValue,
}

pub fn identifier_type(s: &str) -> TokenType {
    use TokenType::*;
    match s {
        "and" => And,
        "class" => Class,
        "else" => Else,
        "false" => False,
        "for" => For,
        "fun" => Fun,
        "if" => If,
        "nil" => Nil,
        "or" => Or,
        "print" => Print,
        "return" => Return,
        "super" => Super,
        "this" => This,
        "true" => True,
        "var" => Var,
        "while" => While,
        _ => Identifier,
    }
}
