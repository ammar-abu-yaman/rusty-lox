use crate::token::{Literal, Token, TokenType};

pub fn error_unkown_symbol(line: u64, s: impl Into<String>) {
    eprintln!("[line {}] Error: Unexpected character: {}", line, s.into());
}

pub fn error(line: u64, err: impl Into<&'static str>) {
    eprintln!("[line {line}] Error: {}", err.into());
}

pub fn token(token: &Token) {
    println!(
        "{} {} {}",
        token_name(token),
        token.lexeme,
        token_value(token)
    )
}

pub fn token_value(token: &Token) -> String {
    use Literal::*;
    match &token.literal {
        String(s) => s.clone(),
        Number(n) => format!("{n:?}"),
        NoValue => "null".to_string(),
    }
}

pub const fn token_name(token: &Token) -> &'static str {
    use TokenType::*;
    match token.token_type {
        LeftParen => "LEFT_PAREN",
        RightParen => "RIGHT_PAREN",
        LeftBrace => "LEFT_BRACE",
        RightBrace => "RIGHT_BRACE",
        Plus => "PLUS",
        Minus => "MINUS",
        Dot => "DOT",
        SemiColon => "SEMICOLON",
        Star => "STAR",
        Comma => "COMMA",
        Asign => "EQUAL",
        Equal => "EQUAL_EQUAL",
        Eof => "EOF",
        Not => "BANG",
        NotEqual => "BANG_EQUAL",
        Less => "LESS",
        LessEq => "LESS_EQUAL",
        Greater => "GREATER",
        GreaterEq => "GREATER_EQUAL",
        Div => "SLASH",
        String => "STRING",
        Number => "NUMBER",
        Identifier => "IDENTIFIER",
        And => "AND",
        Class => "CLASS",
        Else => "ELSE",
        False => "FALSE",
        For => "FOR",
        Fun => "FUN",
        If => "IF",
        Nil => "NIL",
        Or => "OR",
        Print => "PRINT",
        Return => "RETURN",
        Super => "SUPER",
        This => "THIS",
        True => "TRUE",
        Var => "VAR",
        While => "WHILE",
    }
}
