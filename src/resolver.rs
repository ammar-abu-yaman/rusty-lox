use std::collections::HashMap;

use crate::log;
use crate::syntax::{BlockStatement, ClassDecl, Expr, ExpressionStatement, FunctionDecl, IfStatemnet, PrintStatement, ReturnStatement, Statement, VariableDecl, WhileStatement};
use crate::token::Token;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScopeType { Function, Normal }

pub struct Resolver<'a> {
    scopes: Vec<HashMap<&'a str, bool>>,
    current_scope: ScopeType,
    has_err: bool
}

impl <'a> Resolver<'a> {
    pub fn new() -> Self {
        Self {
            scopes: vec![],
            current_scope: ScopeType::Normal,
            has_err: false,
        }
    }

    pub fn has_err(&self) -> bool {
        self.has_err
    }
}

impl Default for Resolver<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl <'a> Resolver<'a> {
    pub fn resolve_stmt(&mut self, stmt: &'a mut Statement) {
        match stmt {
            Statement::VarDecl(var_decl) => self.resolve_var_decl(var_decl),
            Statement::Print(print_statement) => self.resolve_print_stmt(print_statement),
            Statement::Block(block_statement) => self.resolve_block_stmt(block_statement),
            Statement::Expr(expression_statement) => self.resolve_expr_stmt(expression_statement),
            Statement::If(if_statement) => self.resolve_if_stmt(if_statement),
            Statement::While(while_statement) => self.resolve_while_stmt(while_statement),
            Statement::FunDecl(func_decl) => self.resolve_fun_decl(func_decl),
            Statement::Return(return_statement) => self.resolve_return_stmt(return_statement),
            Statement::ClassDecl(class_decl) => self.resolve_class_decl(class_decl)
        }
    }

    fn resolve_class_decl(&mut self, stmt: &'a mut ClassDecl) {
        self.declare(&stmt.name);
        self.define(&stmt.name.lexeme);
    }

    fn resolve_var_decl(&mut self, stmt: &'a mut VariableDecl) {
        self.declare(&stmt.name);
        if let Some(initializer) = &mut stmt.initializer {
            self.resolve_expr(initializer);
        }
        self.define(&stmt.name.lexeme);
    }

    fn resolve_block_stmt(&mut self, stmt: &'a mut BlockStatement) {
        self.begin_scope();
        for statement in &mut stmt.statements {
            self.resolve_stmt(statement);
        }
        self.end_scope();
    }

    fn resolve_print_stmt(&mut self, stmt: &'a mut PrintStatement) {
        self.resolve_expr(&mut stmt.expr);
    }

    fn resolve_expr_stmt(&mut self, stmt: &'a mut ExpressionStatement) {
        self.resolve_expr(&mut stmt.expr);
    }

    fn resolve_if_stmt(&mut self, stmt: &'a mut IfStatemnet) {
        self.resolve_expr(&mut stmt.condition);
        self.resolve_stmt(&mut stmt.if_branch);
        if let Some(else_branch) = &mut stmt.else_branch {
            self.resolve_stmt(else_branch);
        }
    }

    fn resolve_while_stmt(&mut self, stmt: &'a mut WhileStatement) {
        self.resolve_expr(&mut stmt.condition);
        self.resolve_stmt(&mut stmt.body);
    }

    fn resolve_fun_decl(&mut self, stmt: &'a mut FunctionDecl) {
        self.declare(&stmt.name);
        self.define(&stmt.name.lexeme);
        self.resolve_function(&mut stmt.params, &mut stmt.body, ScopeType::Function);
    }   

    fn resolve_function(&mut self, params: &'a Vec<Token>, stmts: &'a mut Vec<Statement>, scope_type: ScopeType) {
        let old_scope = self.current_scope;
        self.current_scope = scope_type;
        self.begin_scope();
        for param in params {
            self.declare(&param);
            self.define(&param.lexeme);
        }
        stmts.iter_mut().for_each(|stmt| self.resolve_stmt(stmt));
        self.end_scope();
        self.current_scope = old_scope;        
    }

    fn resolve_return_stmt(&mut self, stmt: &'a mut ReturnStatement) {
        if self.current_scope == ScopeType::Normal {
            self.has_err = true;
            log::error_token(&stmt.return_token, "Can't return from top-level code.");
        }
        if let Some(value) = &mut stmt.value {
            self.resolve_expr(value);
        }
    }
}   

impl <'a> Resolver<'a> {
    pub fn resolve_expr(&mut self, expr: &'a mut Expr) {
        match expr {
            Expr::Variable { name, height} => {
                        if self.scopes.last().map(|s| s.get(&name.lexeme[..]) == Some(&false)).unwrap_or(false) {
                            self.has_err = true;
                            log::error_token(name, "Can't read local variable in its own initializer.");
                        }
                        self.annotate(&name.lexeme, height);
                    },
            Expr::Asign { name, value, height } => {
                        self.resolve_expr(value);
                        self.annotate(&name.lexeme, height);
                    },
            Expr::Unary { expr, .. } | Expr::Grouping(expr) => self.resolve_expr(expr),
            Expr::LogicalOr { left, right } 
                    | Expr::LogicalAnd { left, right } 
                    | Expr::Binary { left, right, .. } => {
                        self.resolve_expr(left);
                        self.resolve_expr(right);
                    },
            Expr::Call { callee, args, .. } => {
                        self.resolve_expr(callee);
                        args.iter_mut().for_each(|arg| self.resolve_expr(arg));
                    },
            Expr::Get { object, .. } => self.resolve_expr(object),
            Expr::Set { object, value, .. } => {
                self.resolve_expr(value);
                self.resolve_expr(object);
            }
            Expr::Literal(_) => {},
        }
    }
}



impl <'a> Resolver<'a> {
    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &'a Token) {
        match self.scopes.last_mut()  {
            Some(scope) => { 
                if scope.contains_key(&name.lexeme[..]) {
                    self.has_err = true;
                    log::error_token(name, "Already a variable with this name in this scope.");
                } 
                scope.insert(&name.lexeme, false); 
            },
            None => {},
        }
    }

    fn define(&mut self, name: &'a str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.entry(name).and_modify(|b| *b = true);
        }
    }

    fn annotate(&mut self, name: &str, height: &mut Option<usize>) {
        if let Some((index, _)) = self.scopes.iter()
            .rev()
            .enumerate()
            .find(|(_, s)| s.contains_key(name)) {
                height.insert(index);
        }
    }
}