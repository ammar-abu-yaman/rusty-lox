use std::fmt::{write, Display};

use crate::token::{Token, TokenType};


pub struct Ast {
    pub root: Expr,
}

impl Ast {
    pub fn new(root: Expr) -> Self {
        Self { root }
    }
}

pub type BoxedExpr = Box<Expr>;

pub enum Expr {
    Binary { left: BoxedExpr, operator: Token, right: BoxedExpr },
    Unary { operator: Token, expr: BoxedExpr },
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
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Binary { left, operator, right } => unimplemented!(),
            Expr::Unary { operator, expr } => unimplemented!(),
            Expr::Grouping(expr) => write!(f, "(group {expr})"),
            Expr::Nil => write!(f, "nil"),
            Expr::Bool(b) => write!(f, "{b}"),
            Expr::String(s) => write!(f, "{s}"),
            Expr::Number(n) => write!(f, "{n:?}"),
        }
    }
}