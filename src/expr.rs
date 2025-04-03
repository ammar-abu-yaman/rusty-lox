use crate::token::Token;

pub type BoxedExpr = Box<Expr>;

pub enum Expr {
    Binary { left: BoxedExpr, operator: Token, right: BoxedExpr },
    Unary { operator: Token, expr: BoxedExpr },
    Literal(Token),
    Grouping(BoxedExpr)
}


use Expr::*;

impl Expr {
    pub fn binary(left: Expr, operator: Token, right: Expr) -> Self {
        Binary { left: BoxedExpr::new(left), operator, right: BoxedExpr::new(right) }
    }

    pub fn Unary(operator: Token, expr: Expr) -> Self {
        Unary { operator, expr: BoxedExpr::new(expr) }
    }

    pub fn literal(token: Token) -> Self {
        Literal(token)
    }

    pub fn grouping(expr: Expr) -> Self {
        Grouping(BoxedExpr::new(expr))
    }
}