use crate::token::Token;
use std::io::Write;

pub struct Logger<W: Write> {
    writer: W,
}

impl<W: Write> Logger<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn log_err_at(&mut self, token: &Token, message: &str) {
        use crate::token::TokenType;
        let _ = write!(self.writer, "[line {}] Error ", token.pos.line);
        match token.token_type {
            TokenType::Eof => {
                let _ = write!(self.writer, "at End");
            }
            TokenType::Error => {}
            _ => {
                let _ = write!(self.writer, "at '{}'", token.lexeme);
            }
        }

        let _ = writeln!(self.writer, ": {}", message);
    }
}
