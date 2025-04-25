use super::{data::{Result, RuntimeError, Variable}, env::Environment, Evaluator, Interpreter};

use crate::{
    syntax::{Ast, BlockStatement, DeclarationStatement, Expr, ExpressionStatement, IfStatemnet, PrintStatement, Statement, Value},
    token::{Token, TokenType},
};

pub struct TreeWalk {
    environment: Environment,
}

impl TreeWalk {
    pub fn new() -> Self {
        Self {
            environment: Environment::new(),
        }
    }
}

impl Default for TreeWalk {
    fn default() -> Self {
        Self::new()
    }
}

impl Evaluator for TreeWalk {
    fn eval(&mut self, expr: &Expr) -> Result<Value> {
        self.eval_expr(expr)
    }
}

impl Interpreter for TreeWalk {
    fn interpret(&mut self, ast: Ast) -> Result<()> {
        for statement in &ast.statements {
            self.eval_stmt(&statement)?;
        }
        Ok(())
    }
}

impl TreeWalk {
    fn eval_stmt(&mut self, statement: &Statement) -> Result<()> {
        match statement {
            Statement::Decl(declaration_statement) => self.eval_decl_stmt(declaration_statement),
            Statement::Print(print_statement) => self.eval_print_stmt(print_statement),
            Statement::Block(block_statement) => self.eval_block_stmt(block_statement),
            Statement::Expression(expression_statement) => self.eval_expr_stmt(expression_statement),
            Statement::If(if_statement) => self.eval_if_stmt(if_statement),
        }
    }

    fn eval_decl_stmt(&mut self, stmt: &DeclarationStatement) -> Result<()> {
        let name = stmt.name.lexeme.clone();
        let value = match &stmt.initializer {
            Some(initializer) => self.eval_expr(initializer)?,
            None => Value::Nil,
            
        };
        self.environment.define(name.clone(), Variable {
            token: stmt.name.clone(),
            name,
            value,
        });
        Ok(())
    }

    fn eval_print_stmt(&mut self, stmt: &PrintStatement) -> Result<()> {
        let value = self.eval_expr(&stmt.expr)?;
        println!("{}", value);
        Ok(())
    }

    fn eval_block_stmt(&mut self, stmt:&BlockStatement) -> Result<()> {
        self.environment.push_env();
        for statement in &stmt.statements {
            match self.eval_stmt(statement) {
                Ok(()) => continue,
                err @ Err(_) => {
                    self.environment.pop_env();
                    return err
                }, 
            }
        }
        self.environment.pop_env();
        Ok(())
    }
    
    fn eval_expr_stmt(&mut self, stmt: &ExpressionStatement) -> Result<()> {
        self.eval_expr(&stmt.expr)?;
        Ok(())
    }

    fn eval_if_stmt(&mut self, stmt: &IfStatemnet) -> Result<()> {
        let condition_result = self.eval_expr(&stmt.condition)?;
        if is_true(&condition_result) {
            self.eval_stmt(&stmt.if_branch)?;
        } else if let Some(stmt) = &stmt.else_branch {
            self.eval_stmt(&stmt)?;
        }
        Ok(())
    }
    
    fn eval_expr(&mut self, expr: &Expr) -> Result<Value> {
        match expr {
            Expr::Asign { name, value } => {
                let value = self.eval_expr(value)?;
                let variable = Variable {
                    token: name.clone(),
                    name: name.lexeme.clone(),
                    value: value.clone(),
                };
                self.environment.assign(name.clone(), variable)?;
                Ok(value)
            },
            Expr::Binary {
                left,
                operator,
                right,
            } => self.eval_binary(left, operator, right),
            Expr::Unary { operator, expr } => self.eval_unary(operator, expr),
            Expr::Grouping(expr) => self.eval_expr(expr),
            Expr::Literal(value) => Ok(value.clone()),
            Expr::Variable(token) => {
                match self.environment.get(&token.lexeme) {
                    Some(var) => Ok(var.value.clone()),
                    None => Err(RuntimeError::UndefinedVariable { token: token.clone() })
                }
            },
            Expr::LogicalOr { left, right } => self.eval_or(left, right),
            Expr::LogicalAnd { left, right } => self.eval_and(left, right),
        }
    }

    fn eval_or(&mut self, left: &Expr, right: &Expr) -> Result<Value> {
        let left_value = self.eval_expr(left)?;
        if is_true(&left_value) {
            return Ok(left_value);
        } else {
            return Ok(self.eval_expr(right)?);
        }
    }

    fn eval_and(&mut self, left: &Expr, right: &Expr) -> Result<Value> {
        let left_value = self.eval_expr(left)?;
        if !is_true(&left_value) {
            return Ok(left_value);
        } else {
            return Ok(self.eval_expr(right)?);
        }
    }
    
    fn eval_binary(&mut self, left: &Expr, operator: &Token, right: &Expr) -> Result<Value> {
        let left_value = self.eval_expr(left)?;
        let right_value = self.eval_expr(right)?;
        use TokenType::*;
        match (left_value, operator.token_type, right_value) {
            // Arithmetic operations
            (Value::Number(l), Plus, Value::Number(r)) => Ok(Value::Number(l + r)),
            (Value::Number(l), Minus, Value::Number(r)) => Ok(Value::Number(l - r)),
            (Value::Number(l), Star, Value::Number(r)) => Ok(Value::Number(l * r)),
            (Value::Number(l), Div, Value::Number(r)) => Ok(Value::Number(l / r)),
            (Value::Number(l), Greater, Value::Number(r)) => Ok(Value::Bool(l > r)),
            (Value::Number(l), GreaterEq, Value::Number(r)) => Ok(Value::Bool(l >= r)),
            (Value::Number(l), Less, Value::Number(r)) => Ok(Value::Bool(l < r)),
            (Value::Number(l), LessEq, Value::Number(r)) => Ok(Value::Bool(l <= r)),
    
            // String operations
            (Value::String(l), Plus, Value::String(r)) => Ok(Value::String(format!("{}{}", l, r))),
    
            // Logical operations
            (Value::Bool(l), And, Value::Bool(r)) => Ok(Value::Bool(l && r)),
            (Value::Bool(l), Or, Value::Bool(r)) => Ok(Value::Bool(l || r)),
    
            // Equality operations
            (l, Equal, r) => Ok(Value::Bool(l == r)),
            (l, NotEqual, r) => Ok(Value::Bool(l != r)),
    
            // Incompatible types
            (_, Plus | Minus | Div | Star | Greater | GreaterEq | Less | LessEq, _) => {
                Err(RuntimeError::IncompatibleOperandType {
                    operator: operator.clone(),
                    message: "Operands must be numbers".to_string(),
                })
            }
    
            _ => panic!("Invalid binary operation"),
        }
    }
    
    fn eval_unary(&mut self, operator: &Token, expr: &Expr) -> Result<Value> {
        let value = self.eval_expr(expr)?;
        match operator.token_type {
            TokenType::Minus => match value {
                Value::Number(n) => Ok(Value::Number(-n)),
                _ => Err(RuntimeError::IncompatibleOperandType {
                    operator: operator.clone(),
                    message: "Operand must be a number".to_string(),
                }),
            },
            TokenType::Not => Ok(Value::Bool(!is_true(&value))),
            _ => panic!("Invalid unary operator"),
        }
    }
}

const fn is_true(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        Value::Nil => false,
        _ => true,
    }
}