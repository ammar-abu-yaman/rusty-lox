use crate::syntax::{BlockStatement, Expr, Statement, Value};

mod data;
mod tree_walker;
mod env;

pub use data::{Result, RuntimeError};
pub use env::{BoxedEnvironment, Environment};
pub use tree_walker::TreeWalk;

pub trait Evaluator {
    fn eval(&mut self, expr: &Expr) -> Result<Value>;
}

pub trait Interpreter {
    fn interpret(&mut self, ast: &Statement) -> Result<()>;
    fn interpret_block(&mut self, block: &BlockStatement, env: BoxedEnvironment) -> Result<()>;
}