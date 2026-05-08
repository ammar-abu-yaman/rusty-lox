use std::cell::{Cell, RefCell};

use anyhow::Result;
use thiserror::Error;

use crate::function::FunctionType;
use crate::log;
use crate::scanner::Scanner;
use crate::syntax::*;
use crate::token::{Token, TokenLiteral, TokenType};

pub trait Parser<'t> {
    fn parse(&self, scanner: &'t Scanner) -> Option<Vec<Statement<'t>>>;
    fn parse_expr(&self, scanner: &'t Scanner) -> Option<Expr<'t>>;
}

pub struct RecursiveDecendantParser<'t> {
    tokens: RefCell<Vec<Token<'t>>>,
    current: Cell<usize>,
    has_error: Cell<bool>,
}

#[derive(Error, Debug)]
enum ParseError {
    #[error("Unexpected token")]
    UnexpectedToken,
    #[error("Expression error")]
    ExpressionError,
}

impl RecursiveDecendantParser<'_> {
    pub fn new() -> Self {
        Self {
            tokens: RefCell::new(vec![]),
            current: Cell::new(0),
            has_error: Cell::new(false),
        }
    }
}

impl Default for RecursiveDecendantParser<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'t> Parser<'t> for RecursiveDecendantParser<'t> {
    fn parse(&self, scanner: &'t Scanner) -> Option<Vec<Statement<'t>>> {
        *self.tokens.borrow_mut() = scanner.scan_all();
        let statements = self.program();
        if self.has_error.get() {
            return None;
        }
        Some(statements)
    }

    fn parse_expr(&self, scanner: &'t Scanner) -> Option<Expr<'t>> {
        *self.tokens.borrow_mut() = scanner.scan_all();
        let expr = self.expression();
        if expr.is_err() {
            return None;
        }
        Some(expr.unwrap())
    }
}

impl<'t> RecursiveDecendantParser<'t> {
    fn program(&self) -> Vec<Statement<'t>> {
        let mut statements: Vec<Statement<'t>> = vec![];
        while self.peek().token_type != TokenType::Eof {
            match self.declaration() {
                Ok(stmt) => statements.push(stmt),
                Err(_) => {
                    self.has_error.set(true);
                    self.synchronize();
                },
            }
        }
        statements
    }

    fn declaration(&self) -> Result<Statement<'t>, ParseError> {
        use TokenType::*;
        match self.peek().token_type {
            Var => Ok(Statement::VarDecl(self.variable_declaration()?)),
            Fun => Ok(Statement::FunDecl(self.function_declaration(FunctionType::Function)?)),
            Class => Ok(Statement::ClassDecl(self.class_declaration()?)),
            _ => Ok(self.statement()?),
        }
    }

    fn class_declaration(&self) -> Result<ClassDecl<'t>, ParseError> {
        self.consume(TokenType::Class, "Expect 'class' before class name.")?;
        let name = self.consume(TokenType::Identifier, "Expect class name.")?;
        let superclass = if self.peek().token_type == TokenType::Less {
            self.advance();
            let superclass = self.consume(TokenType::Identifier, "Expect superclass name.")?;
            Some(Expr::variable(superclass, Cell::new(None)))
        } else {
            None
        };

        self.consume(TokenType::LeftBrace, "Expect '{' before class body.")?;
        let mut methods = vec![];
        while !matches!(self.peek().token_type, TokenType::Eof | TokenType::RightBrace) {
            match self.peek().token_type {
                TokenType::Identifier => methods.push(self.function_declaration(FunctionType::Method)?),
                _ => {},
            }
        }
        self.consume(TokenType::RightBrace, "Expect '}' after class body.")?;

        Ok(ClassDecl { name, methods, superclass })
    }

    fn variable_declaration(&self) -> Result<VariableDecl<'t>, ParseError> {
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
        Ok(VariableDecl { name, initializer })
    }

    fn function_declaration(&self, kind: FunctionType) -> Result<FunctionDecl<'t>, ParseError> {
        if matches!(kind, FunctionType::Function) {
            self.consume(TokenType::Fun, format!("Expect 'fun' before function name."))?;
        }
        let name = self.consume(TokenType::Identifier, format!("Expect '{kind}' name."))?;
        self.consume(TokenType::LeftParen, format!("Expect '(' after {kind} name."))?;
        let params = self.parameters()?;
        self.consume(TokenType::RightParen, "message: Expect ')' after parameters.")?;
        let body = self.block_statement(Some(kind))?.statements;
        return Ok(FunctionDecl { name, params, body });
    }

    fn parameters(&self) -> Result<Vec<Token<'t>>, ParseError> {
        let mut params = vec![];
        while self.peek().token_type != TokenType::RightParen {
            if params.len() >= 255 {
                self.has_error.set(true);
                log::error_token(&self.peek(), "Can't have more than 255 parameters.");
            }
            params.push(self.consume(TokenType::Identifier, "Expect parameter name.")?);
            if self.peek().token_type != TokenType::Comma {
                break;
            }
            self.advance();
        }
        Ok(params)
    }

    fn statement(&self) -> Result<Statement<'t>, ParseError> {
        use TokenType::*;
        match self.peek().token_type {
            Print => Ok(Statement::Print(self.print_statement()?)),
            LeftBrace => Ok(Statement::Block(self.block_statement(None)?)),
            If => Ok(Statement::If(self.if_statement()?)),
            While => Ok(Statement::While(self.while_statement()?)),
            For => Ok(self.desugar_for_statement()?),
            Return => Ok(Statement::Return(self.return_statement()?)),
            _ => Ok(Statement::Expr(self.expression_statement()?)),
        }
    }

    fn block_statement(&self, block_type: Option<FunctionType>) -> Result<BlockStatement<'t>, ParseError> {
        let mut statements = vec![];
        self.consume(
            TokenType::LeftBrace,
            match block_type {
                Some(func_type) => format!("Expect '{{' before {func_type} body."),
                None => "Expect '{{' before block.".to_string(),
            },
        )?;
        while !matches!(self.peek().token_type, TokenType::RightBrace | TokenType::Eof) {
            let statement = self.declaration()?;
            statements.push(statement);
        }
        self.consume(TokenType::RightBrace, "Expect '}' after block.")?;
        Ok(BlockStatement { statements })
    }

    fn if_statement(&self) -> Result<IfStatemnet<'t>, ParseError> {
        self.consume(TokenType::If, "Expect 'if' before condition.")?;
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.")?;

        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after condition.")?;
        let if_branch = BoxedStatement::new(self.statement()?);
        let else_branch = match self.peek().token_type {
            TokenType::Else => {
                self.advance();
                Some(BoxedStatement::new(self.statement()?))
            },
            _ => None,
        };
        Ok(IfStatemnet {
            condition,
            if_branch,
            else_branch,
        })
    }

    fn while_statement(&self) -> Result<WhileStatement<'t>, ParseError> {
        self.consume(TokenType::While, "Expect 'while' before condition.")?;
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after condition.")?;
        let body = BoxedStatement::new(self.statement()?);
        Ok(WhileStatement { condition, body })
    }

    fn return_statement(&self) -> Result<ReturnStatement<'t>, ParseError> {
        let return_token = self.advance();
        let value = match self.peek().token_type {
            TokenType::SemiColon => None,
            _ => Some(self.expression()?),
        };
        self.consume(TokenType::SemiColon, "Expect ';' after return value.")?;
        Ok(ReturnStatement { return_token, value })
    }

    fn desugar_for_statement(&self) -> Result<Statement<'t>, ParseError> {
        self.consume(TokenType::For, "Expect 'for' before body.")?;
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.")?;

        let initializer = match self.peek().token_type {
            TokenType::SemiColon => {
                self.advance();
                None
            },
            TokenType::Var => Some(Statement::VarDecl(self.variable_declaration()?)),
            _ => Some(Statement::Expr(self.expression_statement()?)),
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
            Some(expr) => Statement::Block(BlockStatement {
                statements: vec![body, Statement::Expr(ExpressionStatement { expr })],
            }),
            None => body,
        };
        let body = match condition {
            Some(expr) => Statement::While(WhileStatement {
                condition: expr,
                body: BoxedStatement::new(body),
            }),
            None => Statement::While(WhileStatement {
                condition: Expr::literal(Literal::Bool(true)),
                body: BoxedStatement::new(body),
            }),
        };
        let body = match initializer {
            Some(statement) => Statement::Block(BlockStatement {
                statements: vec![statement, body],
            }),
            None => body,
        };
        Ok(body)
    }

    fn print_statement(&self) -> Result<PrintStatement<'t>, ParseError> {
        let print_token = self.advance();
        let expr = self.expression()?;
        self.consume(TokenType::SemiColon, "Expect ';' after value.")?;
        Ok(PrintStatement { print_token, expr })
    }

    fn expression_statement(&self) -> Result<ExpressionStatement<'t>, ParseError> {
        let expr = self.expression()?;
        self.consume(TokenType::SemiColon, "Expect ';' after expression.")?;
        Ok(ExpressionStatement { expr })
    }

    fn expression(&self) -> Result<Expr<'t>, ParseError> {
        self.assignment()
    }

    fn assignment(&self) -> Result<Expr<'t>, ParseError> {
        let expr = self.logical_or()?;
        if self.peek().token_type == TokenType::Asign {
            let equals = self.advance();
            let value = self.assignment()?;
            match expr {
                Expr::Variable { name, .. } => return Ok(Expr::assign(name.clone(), value)),
                Expr::Get { name, object, .. } => return Ok(Expr::set(object, name, value)),
                _ => {
                    self.has_error.set(true);
                    log::error_token(&equals, "Invalid assignment target.");
                },
            }
        }
        Ok(expr)
    }

    fn logical_or(&self) -> Result<Expr<'t>, ParseError> {
        let mut expr = self.logical_and()?;
        while let Token { token_type: TokenType::Or, .. } = self.peek() {
            self.advance();
            let right = self.logical_and()?;
            expr = Expr::or(expr, right);
        }
        Ok(expr)
    }

    fn logical_and(&self) -> Result<Expr<'t>, ParseError> {
        let mut expr = self.equality()?;
        while let Token {
            token_type: TokenType::And, ..
        } = self.peek()
        {
            self.advance();
            let right = self.equality()?;
            expr = Expr::and(expr, right);
        }
        Ok(expr)
    }

    fn equality(&self) -> Result<Expr<'t>, ParseError> {
        use TokenType::*;
        let mut expr = self.comparision()?;
        while let Token {
            token_type: Equal | NotEqual, ..
        } = self.peek()
        {
            let opr = self.advance();
            let right = self.comparision()?;
            expr = Expr::binary(expr, opr, right);
        }
        Ok(expr)
    }

    fn comparision(&self) -> Result<Expr<'t>, ParseError> {
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

    fn term(&self) -> Result<Expr<'t>, ParseError> {
        use TokenType::*;
        let mut expr = self.factor()?;
        while let Token { token_type: Plus | Minus, .. } = self.peek() {
            let opr = self.advance();
            let right = self.factor()?;
            expr = Expr::binary(expr, opr, right);
        }
        Ok(expr)
    }

    fn factor(&self) -> Result<Expr<'t>, ParseError> {
        use TokenType::*;
        let mut expr = self.unary()?;
        while let Token { token_type: Div | Star, .. } = self.peek() {
            let opr = self.advance();
            let right = self.unary()?;
            expr = Expr::binary(expr, opr, right);
        }
        Ok(expr)
    }

    fn unary(&self) -> Result<Expr<'t>, ParseError> {
        use TokenType::*;
        match self.peek() {
            Token { token_type: Not | Minus, .. } => {
                let opr = self.advance();
                let expr = self.unary()?;
                return Ok(Expr::unary(opr, expr));
            },
            _ => self.call(),
        }
    }

    fn call(&self) -> Result<Expr<'t>, ParseError> {
        let mut expr = self.primary()?;
        while matches!(self.peek().token_type, TokenType::Dot | TokenType::LeftParen) {
            match self.advance().token_type {
                TokenType::Dot => {
                    let name = self.consume(TokenType::Identifier, "Expect property name after '.'.")?;
                    expr = Expr::get(expr, name);
                },
                TokenType::LeftParen => {
                    let args = match self.peek().token_type {
                        TokenType::RightParen => vec![],
                        _ => self.arguments()?,
                    };
                    if args.len() >= 255 {
                        self.has_error.set(true);
                        log::error_token(&self.peek(), "Can't have more than 255 arguments.");
                    }
                    let paren = self.consume(TokenType::RightParen, "Expect ')' after arguments.")?;
                    expr = Expr::call(expr, paren, args);
                },
                _ => unreachable!(),
            }
        }

        Ok(expr)
    }

    fn arguments(&self) -> Result<Vec<Expr<'t>>, ParseError> {
        let expr = self.expression()?;
        let mut args = vec![expr];
        while let TokenType::Comma = self.peek().token_type {
            self.advance();
            let expr = self.expression()?;
            args.push(expr);
        }
        Ok(args)
    }

    fn primary(&self) -> Result<Expr<'t>, ParseError> {
        use TokenType::*;
        match self.advance() {
            Token { token_type: Nil, .. } => Ok(Expr::literal(Literal::Nil)),
            Token { token_type: True, .. } => Ok(Expr::literal(Literal::Bool(true))),
            Token { token_type: False, .. } => Ok(Expr::literal(Literal::Bool(false))),
            Token {
                token_type: Number,
                literal: TokenLiteral::Number(n),
                ..
            } => Ok(Expr::literal(Literal::Number(n))),
            Token {
                token_type: String,
                literal: TokenLiteral::String(s),
                ..
            } => Ok(Expr::literal(Literal::String(s))),
            Token { token_type: LeftParen, .. } => {
                let expr = self.expression()?;
                self.consume(RightParen, "Expect ')' after expression.")?;
                Ok(Expr::grouping(expr))
            },
            keyword @ Token { token_type: This, .. } => Ok(Expr::this(keyword)),
            keyword @ Token { token_type: Super, .. } => {
                self.consume(TokenType::Dot, "Expect '.' after 'super'.")?;
                let method = self.consume(TokenType::Identifier, "Expect superclass method name.")?;
                Ok(Expr::super_(keyword, method))
            },
            token @ Token { token_type: Identifier, .. } => Ok(Expr::variable(token.clone(), Cell::new(None))),
            token => {
                log::error_token(&token, "Expect expression.");
                Err(ParseError::ExpressionError)
            },
        }
    }
}

impl<'t> RecursiveDecendantParser<'t> {
    fn peek(&self) -> Token<'t> {
        self.tokens
            .borrow()
            .get(self.current.get())
            .cloned()
            .unwrap_or_else(|| self.tokens.borrow().last().unwrap().clone())
    }

    fn advance(&self) -> Token<'t> {
        let token = self.tokens.borrow().get(self.current.get()).copied();
        if token.is_some() {
            self.current.update(|c| c + 1);
        }
        token.unwrap_or_else(|| self.tokens.borrow().last().unwrap().clone())
    }

    fn consume(&self, tt: TokenType, message: impl Into<String>) -> Result<Token<'t>, ParseError> {
        match self.peek() {
            Token { token_type, .. } if token_type == tt => Ok(self.advance()),
            token => {
                log::error_token(&token, &message.into());
                Err(ParseError::UnexpectedToken)
            },
        }
    }

    fn synchronize(&self) {
        use TokenType::*;
        let mut token = self.advance();
        while token.token_type != Eof {
            if token.token_type == SemiColon {
                return;
            }
            match self.peek().token_type {
                Class | Fun | Var | For | If | While | Print | Return => return,
                _ => {
                    token = self.advance();
                },
            }
        }
    }
}
