use crate::syntax::{Expr, Statement, Value};
use crate::token::Token;

pub mod tree_walker;
pub mod vm;

pub use tree_walker::TreeWalk;

pub use self::tree_walker::env::BoxedEnvironment;

pub trait Evaluator<'a, 't> {
    fn eval(&mut self, expr: &Expr<'t>) -> Result<'a, 't, Value<'a, 't>>;
}

pub trait Interpreter<'a, 't> {
    fn interpret(&mut self, ast: &'a Statement<'t>) -> Result<'a, 't, ()>;
    fn interpret_block(&mut self, block: &'a [Statement<'t>], env: BoxedEnvironment<'a, 't>) -> Result<'a, 't, ()>;
}

use thiserror::Error;

pub type Result<'a, 't, T> = anyhow::Result<T, RuntimeError<'a, 't>>;

#[derive(Error, Debug)]
pub enum RuntimeError<'a, 't> {
    #[error("{message}\n[line {}]", operator.pos.line)]
    IncompatibleOperandType { operator: Token<'t>, message: String },
    #[error("Undefined variable '{}'.\n[line {}]", token.lexeme, token.pos.line)]
    UndefinedVariable { token: Token<'t> },
    #[error("Can only call functions and classes.\n[line {}]", token.pos.line)]
    NotValidCallable { token: Token<'t> },
    #[error("Expected {expected} arguments but got {actual}.\n[line {}]", token.pos.line)]
    InvalidArgumentCount { token: Token<'t>, expected: usize, actual: usize },
    #[error("Only instances have properties.\n[line {}]", token.pos.line)]
    NotAnInstance { token: Token<'t> },
    #[error("Undefined property '{}'.\n[line {}]", token.lexeme, token.pos.line)]
    UndefinedProperty { token: Token<'t> },
    #[error("Superclass must be a class.\n[line {}]", token.pos.line)]
    SuperclassMustBeAClass { token: Token<'t> },
    #[error("")]
    Return(Option<Value<'a, 't>>),
}
