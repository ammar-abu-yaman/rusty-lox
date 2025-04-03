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
        Ok(Ast::new(self.primary()))
    }
}

impl RecursiveDecendantParser {
    fn primary(&mut self) -> Expr {
        use TokenType::*;
        match self.advance() {
            Some(Token { token_type: Nil, .. }) => Expr::Nil,
            Some(Token { token_type: True, .. }) => Expr::Bool(true),
            Some(Token { token_type: False, .. }) => Expr::Bool(false),
            Some(Token { token_type: Number, literal: Literal::Number(n), ..}) => Expr::Number(n),
            _ => panic!("Expected True, False or Nil")
        }
    }
}

impl RecursiveDecendantParser {

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.current).cloned();
        if token.is_some() {
            self.current += 1;
        }
        token
    }
}