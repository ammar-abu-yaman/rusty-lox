use anyhow::Result;
use thiserror::Error;

use crate::{
    log,
    scanner::Scanner,
    syntax::{Ast, DeclarationStatement, Expr, ExpressionStatement, PrintStatement, Statement, Value},
    token::{Literal, Token, TokenType},
};

pub trait LoxParser {
    fn parse(&mut self, scanner: &mut Scanner) -> Option<Ast>;
    fn parse_expr(&mut self, scanner: &mut Scanner) -> Option<Expr>;
}

pub struct RecursiveDecendantParser {
    tokens: Vec<Token>,
    current: usize,
}

#[derive(Error, Debug)]
enum ParseError {
    #[error("Unexpected token")]
    UnexpectedToken,
    #[error("Expression error")]
    ExpressionError,
}

impl RecursiveDecendantParser {
    pub fn new() -> Self {
        Self {
            tokens: vec![],
            current: 0,
        }
    }
}

impl Default for RecursiveDecendantParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LoxParser for RecursiveDecendantParser {
    fn parse(&mut self, scanner: &mut Scanner) -> Option<Ast> {
        self.tokens = scanner.scan_all();
        let statements = self.program();
        if statements.is_err() {
            return None;
        }
        Some(Ast::new(statements.unwrap()))
    }

    fn parse_expr(&mut self, scanner: &mut Scanner) -> Option<Expr> {
        self.tokens = scanner.scan_all();
        let expr = self.expression();
        if expr.is_err() {
            return None;
        }
        Some(expr.unwrap())
    }
}

impl RecursiveDecendantParser {
    fn program(&mut self) -> Result<Vec<Statement>, ParseError> {
        let mut statements = vec![];
        while self.peek().token_type != TokenType::Eof {
            statements.push(self.declaration()?);
        }
        Ok(statements)
    }

    fn declaration(&mut self) -> Result<Statement, ParseError> {
        use TokenType::*;
        match self.peek().token_type {
            Var => Ok(self.decl_statement()?),
            _ => Ok(self.statement()?),
        }
    }

    fn statement(&mut self) -> Result<Statement, ParseError> {
        use TokenType::*;
        match self.peek().token_type {
            Print => Ok(Statement::Print(self.print_statement()?)),
            _ => Ok(Statement::Expression(self.expression_statement()?)),
        }
    }

    fn decl_statement(&mut self) -> Result<Statement, ParseError> {
        self.consume(TokenType::Var, "Expect 'var' before variable name.")?;
        let name = self.consume(TokenType::Identifier, "Expect variable name.")?;
        self.consume(TokenType::Asign, "Expect '=' after variable name.")?;
        let initializer = self.expression()?;
        self.consume(TokenType::SemiColon, "Expect ';' after variable declaration.")?;
        Ok(Statement::Decl(DeclarationStatement { name, initializer }))
    }

    fn print_statement(&mut self) -> Result<PrintStatement, ParseError> {
        let print_token = self.advance();
        let expr = self.expression()?;
        self.consume(TokenType::SemiColon, "Expect ';' after expression.")?;
        Ok(PrintStatement { print_token, expr })
    }

    fn expression_statement(&mut self) -> Result<ExpressionStatement, ParseError> {
        let expr = self.expression()?;
        self.consume(TokenType::SemiColon, "Expect ';' after expression.")?;
        Ok(ExpressionStatement { expr })
    }

    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.equality()
    }

    fn equality(&mut self) -> Result<Expr, ParseError> {
        use TokenType::*;
        let mut expr = self.comparision()?;
        while let Token {
            token_type: Equal | NotEqual,
            ..
        } = self.peek()
        {
            let opr = self.advance();
            let right = self.comparision()?;
            expr = Expr::binary(expr, opr, right);
        }
        Ok(expr)
    }

    fn comparision(&mut self) -> Result<Expr, ParseError> {
        use TokenType::*;
        let mut expr = self.term()?;
        while let Token {
            token_type: Greater | GreaterEq | Less | LessEq,
            ..
        } = self.peek()
        {
            let opr = self.advance();
            let right = self.term()?;
            expr = Expr::binary(expr, opr, right);
        }
        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        use TokenType::*;
        let mut expr = self.factor()?;
        while let Token {
            token_type: Plus | Minus,
            ..
        } = self.peek()
        {
            let opr = self.advance();
            let right = self.factor()?;
            expr = Expr::binary(expr, opr, right);
        }
        Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr, ParseError> {
        use TokenType::*;
        let mut expr = self.unary()?;
        while let Token {
            token_type: Div | Star,
            ..
        } = self.peek()
        {
            let opr = self.advance();
            let right = self.unary()?;
            expr = Expr::binary(expr, opr, right);
        }
        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        use TokenType::*;
        match self.peek() {
            Token {
                token_type: Not | Minus,
                ..
            } => {
                let opr = self.advance();
                let expr = self.unary()?;
                return Ok(Expr::unary(opr, expr));
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        use TokenType::*;
        match self.advance() {
            Token {
                token_type: Nil, ..
            } => Ok(Expr::Literal(Value::Nil)),
            Token {
                token_type: True, ..
            } => Ok(Expr::Literal(Value::Bool(true))),
            Token {
                token_type: False, ..
            } => Ok(Expr::Literal(Value::Bool(false))),
            Token {
                token_type: Number,
                literal: Literal::Number(n),
                ..
            } => Ok(Expr::Literal(Value::Number(n))),
            Token {
                token_type: String,
                literal: Literal::String(s),
                ..
            } => Ok(Expr::Literal(Value::String(s))),
            Token {
                token_type: LeftParen,
                ..
            } => {
                let expr = self.expression()?;
                self.consume(RightParen, "Expect ')' after expression.")?;
                Ok(Expr::grouping(expr))
            },
            token @ Token {
                token_type: Identifier,
                ..
            } => Ok(Expr::Identifier(token.clone())),
            token => {
                log::error_token(&token, "Expected expression.");
                Err(ParseError::ExpressionError)
            }
        }
    }
}

impl RecursiveDecendantParser {
    fn peek(&self) -> &Token {
        self.tokens
            .get(self.current)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens.get(self.current).cloned();
        if token.is_some() {
            self.current += 1;
        }
        token.unwrap_or_else(|| self.tokens.last().unwrap().clone())
    }

    fn consume(&mut self, tt: TokenType, message: impl Into<String>) -> Result<Token, ParseError> {
        match self.peek() {
            Token { token_type, .. } if token_type == &tt => Ok(self.advance()),
            token => {
                log::error_token(token, &message.into());
                Err(ParseError::UnexpectedToken)
            }
        }
    }
}
