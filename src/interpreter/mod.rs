use crate::{syntax::{BlockStatement, Expr, Statement, Value}, token::Token};

mod tree_walker;

pub use super::env::{BoxedEnvironment, Environment};
pub use tree_walker::TreeWalk;

pub trait Evaluator {
    fn eval(&mut self, expr: &Expr) -> Result<Value>;
}

pub trait Interpreter {
    fn interpret(&mut self, ast: &Statement) -> Result<()>;
    fn interpret_block(&mut self, block: &BlockStatement, env: BoxedEnvironment) -> Result<()>;
}

use thiserror::Error;

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
    #[error("Only instances have properties.\n[line {}]", token.pos.line)]
    NotAnInstance { token: Token },
    #[error("Undefined property '{}'.\n[line {}]", token.lexeme, token.pos.line)]
    UndefinedProperty { token: Token },
    #[error("Superclass must be a class.\n[line {}]", token.pos.line)]
    SuperclassMustBeAClass { token: Token },
    #[error("")]
    Return(Option<Value>),
}