use crate::{
    syntax::{Ast, Expr, Value},
    token::{Token, TokenType},
};

pub fn eval(ast: Ast) -> Value {
    eval_expr(&ast.root)
}

fn eval_expr(expr: &Expr) -> Value {
    match expr {
        Expr::Binary {
            left,
            operator,
            right,
        } => eval_binary(left, operator, right),
        Expr::Unary { operator, expr } => eval_unary(operator, expr),
        Expr::Grouping(expr) => eval_expr(expr),
        Expr::Literal(value) => value.clone(),
    }
}

fn eval_binary(left: &Expr, operator: &Token, right: &Expr) -> Value {
    let left_value = eval_expr(left);
    let right_value = eval_expr(right);
    use TokenType::*;
    match (left_value, operator.token_type, right_value) {
        // Arithmetic operations
        (Value::Number(l), Plus, Value::Number(r)) => Value::Number(l + r),
        (Value::Number(l), Minus, Value::Number(r)) => Value::Number(l - r),
        (Value::Number(l), Star, Value::Number(r)) => Value::Number(l * r),
        (Value::Number(l), Div, Value::Number(r)) => Value::Number(l / r),
        (Value::Number(l), Greater, Value::Number(r)) => Value::Bool(l > r),
        (Value::Number(l), GreaterEq, Value::Number(r)) => Value::Bool(l >= r),
        (Value::Number(l), Less, Value::Number(r)) => Value::Bool(l < r),
        (Value::Number(l), LessEq, Value::Number(r)) => Value::Bool(l <= r),

        // String operations
        (Value::String(l), Plus, Value::String(r)) => Value::String(format!("{}{}", l, r)),

        // Logical operations
        (Value::Bool(l), And, Value::Bool(r)) => Value::Bool(l && r),
        (Value::Bool(l), Or, Value::Bool(r)) => Value::Bool(l || r),

        // Equality operations
        (l, Equal, r) => Value::Bool(l == r),
        (l, NotEqual, r) => Value::Bool(l != r),
        _ => panic!("Invalid binary operation"),
    }
}

fn eval_unary(operator: &Token, expr: &Expr) -> Value {
    let value = eval_expr(expr);
    match operator.token_type {
        TokenType::Minus => match value {
            Value::Number(n) => Value::Number(-n),
            _ => panic!("Invalid unary operator"),
        },
        TokenType::Not => Value::Bool(!is_true(&value)),
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
