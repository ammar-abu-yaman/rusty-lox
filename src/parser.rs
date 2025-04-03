use crate::{syntax::{Ast, Expr}, token::{Literal, Token, TokenType}};
use std::{io::Result, usize};

pub trait LoxParser {
    fn parse<T: IntoIterator<Item = Result<Token>>>(&mut self, tokens: T) -> Result<Ast>;
}

pub struct RecursiveDecendantParser {
    tokens: Vec<Token>,
    current: usize,
}

impl RecursiveDecendantParser {
    pub fn new() -> Self {
        Self { tokens: vec![], current: 0 }
    }
}

impl Default for RecursiveDecendantParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LoxParser for RecursiveDecendantParser {
    fn parse<T: IntoIterator<Item = Result<Token>>>(&mut self, tokens: T) -> Result<Ast> {
        self.tokens = tokens.into_iter()
            .filter(|token| token.is_ok())
            .map(Result::unwrap)
            .collect();     
        Ok(Ast::new(self.expression()))
    }
}

impl RecursiveDecendantParser {

    fn expression(&mut self) -> Expr {
        self.factor()
    }

    fn factor(&mut self) -> Expr {
        use TokenType::*;
        let mut expr = self.unary();
        while let Some(Token { token_type: Div | Star, ..}) = self.peek() {
            let opr = self.advance().unwrap();
            let right = self.unary();
            expr = Expr::binary(expr, opr, right);
        }
        expr
    }

    fn unary(&mut self) -> Expr {
        use TokenType::*;
        match self.peek() {
            Some(Token { token_type: Not | Minus, ..}) => {
                let opr = self.advance().unwrap();
                let expr = self.unary();
                return Expr::unary(opr, expr)
            },
            Some(_) => self.primary(),
            None => panic!("Expected an unary expression"),
        }
    }

    fn primary(&mut self) -> Expr {
        use TokenType::*;
        match self.advance() {
            Some(Token { token_type: Nil, .. }) => Expr::Nil,
            Some(Token { token_type: True, .. }) => Expr::Bool(true),
            Some(Token { token_type: False, .. }) => Expr::Bool(false),
            Some(Token { token_type: Number, literal: Literal::Number(n), ..}) => Expr::Number(n),
            Some(Token { token_type: String, literal: Literal::String(s), ..}) => Expr::String(s),
            Some(Token { token_type: LeftParen, ..}) => {
                let expr = self.expression();
                self.consume(RightParen);
                Expr::grouping(expr)
            }
            _ => panic!("Expected True, False, Number, String or Nil")
        }
    }
}

impl RecursiveDecendantParser {

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current)
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.current).cloned();
        if token.is_some() {
            self.current += 1;
        }
        token
    }

    fn consume(&mut self, tt: TokenType) {
        match self.advance() {
            Some(Token { token_type, .. }) if token_type == tt => {},
            _ => panic!("Unexpected token type"),
        }
    }
}