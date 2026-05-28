use crate::token::Token;

pub struct Logger;

impl Logger {
    pub fn log_err_at(&self, token: &Token, message: &str) {
        use crate::token::TokenType;
        eprintln!("[line {}] Error ", token.pos.line);
        match token.token_type {
            TokenType::Eof => eprint!("at End"),
            TokenType::Error => {},
            _ => eprint!("at {}", token.pos.offset),
        }

        eprintln!(": {}", message);
    }
}
