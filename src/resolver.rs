use std::cell::Cell;
use std::collections::HashMap;
use std::mem;

use crate::log;
use crate::syntax::*;
use crate::token::Token;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScopeType {
    Function,
    Method,
    Initializer,
    Normal,
}

enum ClassType {
    Class,
    Subclass,
    None,
}

pub struct Resolver<'a> {
    scopes: Vec<HashMap<&'a str, bool>>,
    current_scope: ScopeType,
    current_class: ClassType,
    has_err: bool,
}

impl<'a> Resolver<'a> {
    pub fn new() -> Self {
        Self {
            scopes: vec![],
            current_scope: ScopeType::Normal,
            has_err: false,
            current_class: ClassType::None,
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

impl<'a> Resolver<'a> {
    pub fn resolve_stmt(&mut self, stmt: &'a Statement) {
        match stmt {
            Statement::VarDecl(var_decl) => self.resolve_var_decl(var_decl),
            Statement::Print(print_statement) => self.resolve_print_stmt(print_statement),
            Statement::Block(block_statement) => self.resolve_block_stmt(block_statement),
            Statement::Expr(expression_statement) => self.resolve_expr_stmt(expression_statement),
            Statement::If(if_statement) => self.resolve_if_stmt(if_statement),
            Statement::While(while_statement) => self.resolve_while_stmt(while_statement),
            Statement::FunDecl(func_decl) => self.resolve_fun_decl(func_decl),
            Statement::Return(return_statement) => self.resolve_return_stmt(return_statement),
            Statement::ClassDecl(class_decl) => self.resolve_class_decl(class_decl),
        }
    }

    fn resolve_class_decl(&mut self, stmt: &'a ClassDecl) {
        let mut previos_class = ClassType::Class;
        mem::swap(&mut self.current_class, &mut previos_class);
        self.declare(&stmt.name);
        self.define(&stmt.name.lexeme);

        if let Some(super_expr @ Expr::Variable { name, .. }) = &stmt.superclass {
            if name.lexeme == stmt.name.lexeme {
                self.has_err = true;
                log::error_token(name, "A class can't inherit from itself.");
            }
            self.current_class = ClassType::Subclass;
            self.resolve_expr(&super_expr);
            self.begin_scope();
            self.scopes.last_mut().unwrap().insert("super", true);
        }

        self.begin_scope();
        self.scopes.last_mut().unwrap().insert("this", true);
        for FunctionDecl { name, params, body } in &stmt.methods {
            let method_scope = match &name.lexeme[..] {
                "init" => ScopeType::Initializer,
                _ => ScopeType::Method,
            };
            self.resolve_function(params, body, method_scope);
        }

        self.end_scope();
        if stmt.superclass.is_some() {
            self.end_scope();
        }
        self.current_class = previos_class;
    }

    fn resolve_var_decl(&mut self, stmt: &'a VariableDecl) {
        self.declare(&stmt.name);
        if let Some(initializer) = &stmt.initializer {
            self.resolve_expr(initializer);
        }
        self.define(&stmt.name.lexeme);
    }

    fn resolve_block_stmt(&mut self, stmt: &'a BlockStatement) {
        self.begin_scope();
        for statement in &stmt.statements {
            self.resolve_stmt(statement);
        }
        self.end_scope();
    }

    fn resolve_print_stmt(&mut self, stmt: &'a PrintStatement) {
        self.resolve_expr(&stmt.expr);
    }

    fn resolve_expr_stmt(&mut self, stmt: &'a ExpressionStatement) {
        self.resolve_expr(&stmt.expr);
    }

    fn resolve_if_stmt(&mut self, stmt: &'a IfStatemnet) {
        self.resolve_expr(&stmt.condition);
        self.resolve_stmt(&stmt.if_branch);
        if let Some(else_branch) = &stmt.else_branch {
            self.resolve_stmt(else_branch);
        }
    }

    fn resolve_while_stmt(&mut self, stmt: &'a WhileStatement) {
        self.resolve_expr(&stmt.condition);
        self.resolve_stmt(&stmt.body);
    }

    fn resolve_fun_decl(&mut self, stmt: &'a FunctionDecl) {
        self.declare(&stmt.name);
        self.define(&stmt.name.lexeme);
        self.resolve_function(&stmt.params, &stmt.body, ScopeType::Function);
    }

    fn resolve_function(&mut self, params: &'a Vec<Token>, stmts: &'a Vec<Statement>, scope_type: ScopeType) {
        let old_scope = self.current_scope;
        self.current_scope = scope_type;
        self.begin_scope();
        for param in params {
            self.declare(&param);
            self.define(&param.lexeme);
        }
        stmts.iter().for_each(|stmt| self.resolve_stmt(stmt));
        self.end_scope();
        self.current_scope = old_scope;
    }

    fn resolve_return_stmt(&mut self, stmt: &'a ReturnStatement) {
        if self.current_scope == ScopeType::Normal {
            self.has_err = true;
            log::error_token(&stmt.return_token, "Can't return from top-level code.");
        }
        if let Some(value) = &stmt.value {
            if self.current_scope == ScopeType::Initializer {
                self.has_err = true;
                log::error_token(&stmt.return_token, "Can't return a value from an initializer.");
            }
            self.resolve_expr(value);
        }
    }
}

impl<'a> Resolver<'a> {
    pub fn resolve_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Variable { name, height } => {
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
            Expr::LogicalOr { left, right } | Expr::LogicalAnd { left, right } | Expr::Binary { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            },
            Expr::Call { callee, args, .. } => {
                self.resolve_expr(callee);
                args.iter().for_each(|arg| self.resolve_expr(arg));
            },
            Expr::Get { object, .. } => self.resolve_expr(object),
            Expr::Set { object, value, .. } => {
                self.resolve_expr(value);
                self.resolve_expr(object);
            },
            Expr::This { keyword, height } => {
                if matches!(self.current_class, ClassType::None) {
                    self.has_err = true;
                    log::error_token(&keyword, "Can't use 'this' outside of a class.");
                } else {
                    self.annotate(&keyword.lexeme, height)
                }
            },
            Expr::Super { keyword, height, .. } => match self.current_class {
                ClassType::None => {
                    self.has_err = true;
                    log::error_token(&keyword, "Can't use 'super' outside of a class.");
                },
                ClassType::Class => {
                    self.has_err = true;
                    log::error_token(&keyword, "Can't use 'super' in a class with no superclass.");
                },
                ClassType::Subclass => self.annotate(&keyword.lexeme, height),
            },
            Expr::Literal(_) => {},
        }
    }
}

impl<'a> Resolver<'a> {
    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &'a Token) {
        match self.scopes.last_mut() {
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

    fn annotate(&mut self, name: &str, height: &Cell<Option<usize>>) {
        if let Some((index, _)) = self.scopes.iter().rev().enumerate().find(|(_, s)| s.contains_key(name)) {
            height.set(Some(index));
        }
    }
}
