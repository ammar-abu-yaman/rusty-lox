use thiserror::Error;

use crate::{syntax::Value, token::Token};

pub type Result<T> = anyhow::Result<T, RuntimeError>;

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("{message}\n[line {}]", operator.pos.line)]
    IncompatibleOperandType { operator: Token, message: String },
    #[error("Undefined variable '{}'.\n[line {}]", token.lexeme, token.pos.line)]
    UndefinedVariable { token: Token },
}

pub struct Variable {
    pub token: Token,
    pub name: String,
    pub value: Value,
}
