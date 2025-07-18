use crate::syntax::{Expr, Statement, Value};
use crate::token::Token;

mod tree_walker;

pub use tree_walker::TreeWalk;

pub use super::env::{BoxedEnvironment, Environment};

pub trait Evaluator<'a> {
    fn eval(&mut self, expr: &Expr) -> Result<'a, Value<'a>>;
}

pub trait Interpreter<'a> {
    fn interpret(&mut self, ast: &'a Statement) -> Result<'a, ()>;
    fn interpret_block(&mut self, block: &'a [Statement], env: BoxedEnvironment<'a>) -> Result<'a, ()>;
}

use thiserror::Error;

pub type Result<'a, T> = anyhow::Result<T, RuntimeError<'a>>;

#[derive(Error, Debug)]
pub enum RuntimeError<'a> {
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
    Return(Option<Value<'a>>),
}
