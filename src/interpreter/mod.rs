use crate::syntax::{Ast, Expr, Value};

mod data;
mod tree_walker;

pub use data::{Result, RuntimeError};
pub use tree_walker::TreeWalk;

pub trait Evaluator {
    fn eval(&mut self, expr: &Expr) -> Result<Value>;
}

pub trait Interpreter {
    fn interpret(&mut self, ast: Ast) -> Result<()>;
}