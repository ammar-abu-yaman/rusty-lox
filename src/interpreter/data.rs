use thiserror::Error;

use crate::token::Token;

pub type Result<T> = anyhow::Result<T, RuntimeError>;

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("{message}\n[line {}]", operator.pos.line)]
    IncompatibleOperandType { operator: Token, message: String },
    #[error("Undefined variable '{}'.\n[line {}]", token.lexeme, token.pos.line)]
    UndefinedVariable { token: Token },
    #[error("Can only call functions and classes.\n[line {}]", token.pos.line)]
    NotValidCallable { token: Token },
    #[error("Expected {expected} arguments but got {actual}.\n[line {}]", token.pos.line)]
    InvalidArgumentCount { token: Token, expected: usize, actual: usize },
}