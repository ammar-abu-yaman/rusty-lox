use crate::token::Token;

pub fn lex_error(token: &Token) {
    eprintln!("[line {}] Error: Unexpected character: {}", token.pos.line, token.lexeme);
}