

pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub pos: TokenPosition,
}

pub struct TokenPosition {
    line: u64,
}


#[derive(Debug, Clone, Copy)]
pub enum TokenType {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
}
