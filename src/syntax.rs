use std::fmt::Display;

use crate::token::Token;

pub struct Ast {
    pub root: Expr,
}

impl Ast {
    pub fn new(root: Expr) -> Self {
        Self { root }
    }
}

pub type BoxedExpr = Box<Expr>;

#[derive(Debug)]
pub enum Expr {
    Binary {
        left: BoxedExpr,
        operator: Token,
        right: BoxedExpr,
    },
    Unary {
        operator: Token,
        expr: BoxedExpr,
    },
    Grouping(BoxedExpr),
    Nil,
    Bool(bool),
    String(std::string::String),
    Number(f64),
}

impl Expr {
    pub fn grouping(expr: Expr) -> Self {
        Self::Grouping(BoxedExpr::new(expr))
    }

    pub fn unary(operator: Token, expr: Expr) -> Self {
        Self::Unary {
            operator,
            expr: BoxedExpr::new(expr),
        }
    }

    pub fn binary(left: Expr, operator: Token, right: Expr) -> Self {
        Self::Binary {
            left: BoxedExpr::new(left),
            operator,
            right: BoxedExpr::new(right),
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Binary {
                left,
                operator: Token { lexeme, .. },
                right,
            } => write!(f, "({lexeme} {left} {right})"),
            Expr::Unary {
                operator: Token { lexeme, .. },
                expr,
            } => write!(f, "({lexeme} {expr})"),
            Expr::Grouping(expr) => write!(f, "(group {expr})"),
            Expr::Nil => write!(f, "nil"),
            Expr::Bool(b) => write!(f, "{b}"),
            Expr::String(s) => write!(f, "{s}"),
            Expr::Number(n) => write!(f, "{n:?}"),
        }
    }
}
