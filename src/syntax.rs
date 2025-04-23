use std::fmt::Display;

use crate::token::Token;

pub struct Ast {
    pub statements: Vec<Statement>,
}

impl Ast {
    pub fn new(statements: Vec<Statement>) -> Self {
        Self { statements }
    }
}

pub enum Statement {
    Decl(DeclarationStatement),
    Print(PrintStatement),
    Expression(ExpressionStatement),
    Block(BlockStatement),
}

pub type BoxedExpr = Box<Expr>;

#[derive(Debug)]
pub struct DeclarationStatement {
    pub name: Token,
    pub initializer: Option<Expr>,
}

#[derive(Debug)]
pub struct PrintStatement {
    pub print_token: Token,
    pub expr: Expr,
}

pub struct BlockStatement {
    pub statements: Vec<Statement>,
}

#[derive(Debug)]
pub struct ExpressionStatement {
    pub expr: Expr,
}

#[derive(Debug)]
pub enum Expr {
    Asign {
        name: Token,
        value: BoxedExpr,
    },
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
    Literal(Value),
    Variable(Token),

}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    Nil,
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{n}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Nil => write!(f, "nil"),
        }
    }
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

    pub fn variable(identifier: Token) -> Self {
        Self::Variable(identifier)
    }

    pub fn assign(name: Token, value: Expr) -> Self {
        Self::Asign {
            name,
            value: BoxedExpr::new(value),
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Asign { name: Token { lexeme, .. }, value } => {
                write!(f, "(= {lexeme} {value})")
            }
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
            Expr::Literal(Value::Bool(b)) => write!(f, "{b}"),
            Expr::Literal(Value::String(s)) => write!(f, "{s}"),
            Expr::Literal(Value::Nil) => write!(f, "nil"),
            Expr::Literal(Value::Number(n)) => write!(f, "{n:?}"),
            Expr::Variable(Token { lexeme, .. }) => write!(f, "{lexeme}"),
        }
    }
}
