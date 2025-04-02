

pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub pos: TokenPosition,
}

impl Token {
    pub fn new(token_type: TokenType, lexeme: String, line: u64) -> Self {
        Self {
            token_type,
            lexeme,
            pos: TokenPosition { line },
        }
    }
}

impl Token {
    pub fn eof() -> Self {
        Self {
            token_type: TokenType::Eof,
            lexeme: "".to_string(),
            pos: TokenPosition { line: 0 }
        }
    }
}

impl Token {
    pub fn to_token_string(&self) -> String {
        format!("{} {} null", self.token_type.name(), self.lexeme)
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
    Eof,
    Unkown,
}

impl TokenType {
    pub const fn name(&self) -> &'static str {
        match self {
            TokenType::LeftParen => "LEFT_PAREN",
            TokenType::RightParen => "RIGHT_PAREN",
            TokenType::LeftBrace => "LEFT_BRACE",
            TokenType::RightBrace => "RIGHT_BRACE",
            TokenType::Plus => "PLUS",
            TokenType::Minus => "MINUS",
            TokenType::Dot => "DOT",
            TokenType::SemiColon => "SEMICOLON",
            TokenType::Star => "STAR",
            TokenType::Comma => "COMMA",
            TokenType::Asign => "EQUAL",
            TokenType::Equal => "EQUAL_EQUAL",
            TokenType::Eof => "EOF",
            TokenType::Unkown => "UNKOWN",
            TokenType::Not => "BANG",
            TokenType::NotEqual => "BANG_EQUAL",
        }
    }
}