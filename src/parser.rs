use anyhow::Result;
use thiserror::Error;

use crate::{
    log,
    scanner::Scanner,
    syntax::{Ast, BlockStatement, BoxedStatement, DeclarationStatement, Expr, ExpressionStatement, IfStatemnet, PrintStatement, Statement, Value, WhileStatement},
    token::{Literal, Token, TokenType},
};

pub trait LoxParser {
    fn parse(&mut self, scanner: &mut Scanner) -> Option<Ast>;
    fn parse_expr(&mut self, scanner: &mut Scanner) -> Option<Expr>;
}

pub struct RecursiveDecendantParser {
    tokens: Vec<Token>,
    current: usize,
    has_error: bool,
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
            has_error: false,
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
        if self.has_error {
            return None;
        }
        Some(Ast::new(statements))
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
    fn program(&mut self) -> Vec<Statement> {
        let mut statements = vec![];
        while self.peek().token_type != TokenType::Eof {
            match self.declaration() {
                Ok(stmt) => statements.push(stmt),
                Err(_) => {
                    self.has_error = true;
                    self.synchronize();
                },    
            }
        }
        statements
    }

    fn declaration(&mut self) -> Result<Statement, ParseError> {
        use TokenType::*;
        match self.peek().token_type {
            Var => Ok(Statement::Decl(self.decl_statement()?)),
            _ => Ok(self.statement()?),
        }
    }

    fn statement(&mut self) -> Result<Statement, ParseError> {
        use TokenType::*;
        match self.peek().token_type {
            Print => Ok(Statement::Print(self.print_statement()?)),
            LeftBrace => Ok(Statement::Block(self.block_statement()?)),
            If => Ok(Statement::If(self.if_statement()?)),
            While => Ok(Statement::While(self.while_statement()?)),
            For => Ok(self.desugar_for_statement()?),
            _ => Ok(Statement::Expression(self.expression_statement()?)),
        }
    }

    fn decl_statement(&mut self) -> Result<DeclarationStatement, ParseError> {
        self.consume(TokenType::Var, "Expect 'var' before variable name.")?;
        let name = self.consume(TokenType::Identifier, "Expect variable name.")?;
        let initializer = match self.peek().token_type {
            TokenType::Asign => {
                self.advance();
                Some(self.expression()?)
            },
            _ => None,
        };
        self.consume(TokenType::SemiColon, "Expect ';' after variable declaration.")?;
        Ok(DeclarationStatement { name, initializer })
    }

    fn block_statement(&mut self) -> Result<BlockStatement, ParseError> {
        let mut statements = vec![];
        self.consume(TokenType::LeftBrace, "Expect '{' before block.")?;
        while !matches!(self.peek().token_type, TokenType::RightBrace | TokenType::Eof) {
            let statement = self.declaration()?;
            statements.push(statement);
        }
        self.consume(TokenType::RightBrace, "Expect '}' after block.")?;
        Ok(BlockStatement { statements })
    }

    fn if_statement(&mut self) -> Result<IfStatemnet, ParseError> {
        self.consume(TokenType::If, "Expect 'if' before condition.")?;
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.")?;

        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after condition.")?;
        let if_branch = BoxedStatement::new(self.statement()?);
        let else_branch = match self.peek().token_type {
            TokenType::Else => {
                self.advance();
                Some(BoxedStatement::new(self.statement()?))
            }
            _ => None,
        };
        Ok(IfStatemnet {
            condition,
            if_branch,
            else_branch,
        })
    }

    fn while_statement(&mut self) -> Result<WhileStatement, ParseError> {
        self.consume(TokenType::While, "Expect 'while' before condition.")?;
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after condition.")?;
        let body = BoxedStatement::new(self.statement()?);
        Ok(WhileStatement { condition, body })
    }
    
    fn desugar_for_statement(&mut self) -> Result<Statement, ParseError> {
        self.consume(TokenType::For, "Expect 'for' before body.")?;
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.")?;
        
        let initializer = match self.peek().token_type {
            TokenType::SemiColon => {
                self.advance();
                None
            },
            TokenType::Var => Some(Statement::Decl(self.decl_statement()?)),
            _ => Some(Statement::Expression(self.expression_statement()?)),
        };
        let condition = match self.peek().token_type {
            TokenType::SemiColon => None,
            _ => Some(self.expression()?),
        };
        self.consume(TokenType::SemiColon, "Expect ';' after loop condition.")?;
        let increment = match self.peek().token_type {
            TokenType::RightParen => None,
            _ => Some(self.expression()?),
        };
        self.consume(TokenType::RightParen, "Expect ')' after for clauses.")?;

        let body = self.statement()?;
        let body = match increment {
            Some(expr) => Statement::Block(BlockStatement { statements: vec![
                body,
                Statement::Expression(ExpressionStatement { expr }),
            ] }),
            None => body,
        };
        let body = match condition {
            Some(expr) => Statement::While(WhileStatement { condition: expr, body: BoxedStatement::new(body) }),
            None => Statement::While(WhileStatement { condition: Expr::Literal(Value::Bool(true)), body: BoxedStatement::new(body) }),
        };
        let body = match initializer {
            Some(statement) => Statement::Block(BlockStatement { statements: vec![statement, body] }),
            None => body,
        };
        Ok(body)
    }

    fn print_statement(&mut self) -> Result<PrintStatement, ParseError> {
        let print_token = self.advance();
        let expr = self.expression()?;
        self.consume(TokenType::SemiColon, "Expect ';' after value.")?;
        Ok(PrintStatement { print_token, expr })
    }

    fn expression_statement(&mut self) -> Result<ExpressionStatement, ParseError> {
        let expr = self.expression()?;
        self.consume(TokenType::SemiColon, "Expect ';' after expression.")?;
        Ok(ExpressionStatement { expr })
    }

    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.logical_or()?;
        if self.peek().token_type == TokenType::Asign {
            let equals = self.advance();
            let value = self.assignment()?;
            if let Expr::Variable(name) = &expr {
                return Ok(Expr::assign(name.clone(), value));
            }
            self.has_error = true;
            log::error_token(&equals, "Invalid assignment target.");
        }
        Ok(expr)
    }

    fn logical_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.logical_and()?;
        while let Token { token_type: TokenType::Or, .. } = self.peek() {
            self.advance();
            let right = self.logical_and()?;
            expr = Expr::or(expr, right);
        }
        Ok(expr)
    }

    fn logical_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.equality()?;
        while let Token { token_type: TokenType::And, ..} = self.peek() {
            self.advance();
            let right = self.equality()?;
            expr = Expr::and(expr, right);
        }
        Ok(expr)
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
            } => Ok(Expr::Variable(token.clone())),
            token => {
                log::error_token(&token, "Expect expression.");
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

    fn synchronize(&mut self) {
        use TokenType::*;
        let mut token = self.advance();
        while token.token_type != Eof {
            if token.token_type == SemiColon {
                return;
            }
            match self.peek().token_type {
                Class | Fun | Var | For | If | While | Print | Return => return,
                _ => { token = self.advance(); },
            }
        } 
    }
}
