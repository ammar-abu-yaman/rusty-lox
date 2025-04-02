use std::collections::HashMap;

pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub literal: Literal,
    pub pos: TokenPosition,
}

impl Token {
    pub fn new(token_type: TokenType, lexeme: impl Into<String>, literal: Literal, line: u64) -> Self {
        Self {
            token_type,
            lexeme: lexeme.into(),
            literal,
            pos: TokenPosition { line },
        }
    }

    pub fn symbol(token_type: TokenType, lexeme: impl Into<String>, line: u64) -> Self {
        Self::new(token_type, lexeme.into(), Literal::NoValue, line)
    }

    pub fn textual(value: impl Into<String>, line: u64) -> Self {
        let value = value.into();
        Self::new(identifier_type(&value), value, Literal::NoValue, line)
    }

    pub fn string(value: impl Into<String>, line: u64) -> Self {
        let value= value.into();
        Self::new(TokenType::String, format!("\"{}\"", value), Literal::String(value), line)
    }

    pub fn number(value: impl Into<String>, line: u64) -> Self {
        let value = value.into();
        let n = value.parse().unwrap();
        Self::new(TokenType::Number, value, Literal::Number(n), line)
    }
    
    pub fn eof() -> Self {
        Self::new(TokenType::Eof, "", Literal::NoValue, 0)
    }
}

pub struct TokenPosition {
    pub line: u64,
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
pub enum Literal {
    String(String),
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