use thiserror::Error;

use crate::{
    syntax::{Ast, Expr, Value},
    token::{Token, TokenType},
};

type Result<T> = anyhow::Result<T, RuntimeError>;

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("{message}\n[line {}]", operator.pos.line)]
    IncompatibleOperandType { operator: Token, message: String },
}

pub fn eval(ast: Ast) -> Result<Value> {
    eval_expr(&ast.root)
}

fn eval_expr(expr: &Expr) -> Result<Value> {
    match expr {
        Expr::Binary {
            left,
            operator,
            right,
        } => eval_binary(left, operator, right),
        Expr::Unary { operator, expr } => eval_unary(operator, expr),
        Expr::Grouping(expr) => eval_expr(expr),
        Expr::Literal(value) => Ok(value.clone()),
    }
}

fn eval_binary(left: &Expr, operator: &Token, right: &Expr) -> Result<Value> {
    let left_value = eval_expr(left)?;
    let right_value = eval_expr(right)?;
    use TokenType::*;
    match (left_value, operator.token_type, right_value) {
        // Arithmetic operations
        (Value::Number(l), Plus, Value::Number(r)) => Ok(Value::Number(l + r)),
        (Value::Number(l), Minus, Value::Number(r)) => Ok(Value::Number(l - r)),
        (Value::Number(l), Star, Value::Number(r)) => Ok(Value::Number(l * r)),
        (Value::Number(l), Div, Value::Number(r)) => Ok(Value::Number(l / r)),
        (Value::Number(l), Greater, Value::Number(r)) => Ok(Value::Bool(l > r)),
        (Value::Number(l), GreaterEq, Value::Number(r)) => Ok(Value::Bool(l >= r)),
        (Value::Number(l), Less, Value::Number(r)) => Ok(Value::Bool(l < r)),
        (Value::Number(l), LessEq, Value::Number(r)) => Ok(Value::Bool(l <= r)),

        // String operations
        (Value::String(l), Plus, Value::String(r)) => Ok(Value::String(format!("{}{}", l, r))),

        // Logical operations
        (Value::Bool(l), And, Value::Bool(r)) => Ok(Value::Bool(l && r)),
        (Value::Bool(l), Or, Value::Bool(r)) => Ok(Value::Bool(l || r)),

        // Equality operations
        (l, Equal, r) => Ok(Value::Bool(l == r)),
        (l, NotEqual, r) => Ok(Value::Bool(l != r)),

        // Incompatible types
        (_, Plus | Minus | Div | Star | Greater | GreaterEq | Less | LessEq, _) => {
            Err(RuntimeError::IncompatibleOperandType {
                operator: operator.clone(),
                message: "Operands must be numbers".to_string(),
            })
        }

        _ => panic!("Invalid binary operation"),
    }
}

fn eval_unary(operator: &Token, expr: &Expr) -> Result<Value> {
    let value = eval_expr(expr)?;
    match operator.token_type {
        TokenType::Minus => match value {
            Value::Number(n) => Ok(Value::Number(-n)),
            _ => Err(RuntimeError::IncompatibleOperandType {
                operator: operator.clone(),
                message: "Operand must be a number".to_string(),
            }),
        },
        TokenType::Not => Ok(Value::Bool(!is_true(&value))),
        _ => panic!("Invalid unary operator"),
    }
}

const fn is_true(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        Value::Nil => false,
        _ => true,
    }
}
