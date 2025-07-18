use std::cell::Cell;
use std::rc::Rc;

use super::{Evaluator, Interpreter, Result, RuntimeError};
use crate::class::Class;
use crate::env::{BoxedEnvironment, Environment};
use crate::function::{Function, NativeFunction};
use crate::instance::Instance;
use crate::syntax::{
    ClassDecl, Expr, ExpressionStatement, FunctionDecl, IfStatemnet, PrintStatement, ReturnStatement, Statement, Value, VariableDecl,
    WhileStatement,
};
use crate::token::{Token, TokenType};

pub struct TreeWalk<'a> {
    globals: BoxedEnvironment<'a>,
    environment: BoxedEnvironment<'a>,
}

impl TreeWalk<'_> {
    pub fn new() -> Self {
        let globals = Environment::boxed();
        globals
            .borrow_mut()
            .define("clock", Value::NativeFunction(Rc::new(NativeFunction::clock())));
        Self {
            environment: BoxedEnvironment::clone(&globals),
            globals,
        }
    }
}

impl Default for TreeWalk<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl <'a> Evaluator<'a> for TreeWalk<'a> {
    fn eval(&mut self, expr: &Expr) -> Result<'a, Value<'a>> {
        self.eval_expr(expr)
    }
}

impl <'a> Interpreter<'a> for TreeWalk<'a> {
    fn interpret(&mut self, stmt: &'a Statement) -> Result<'a, ()> {
        self.eval_stmt(stmt)?;
        Ok(())
    }

    fn interpret_block(&mut self, block: &'a [Statement], env: BoxedEnvironment<'a>) -> Result<'a, ()> {
        self.eval_block_stmt(block, env)
    }
}

impl <'a> TreeWalk<'a> {
    fn eval_stmt(&mut self, statement: &'a Statement) -> Result<'a, ()> {
        match statement {
            Statement::VarDecl(var_decl) => self.eval_var_decl(var_decl),
            Statement::Print(print_statement) => self.eval_print_stmt(print_statement),
            Statement::Block(block_statement) => self.eval_block_stmt(&block_statement.statements, Environment::boxed_with_enclosing(&self.environment)),
            Statement::Expr(expression_statement) => self.eval_expr_stmt(expression_statement),
            Statement::If(if_statement) => self.eval_if_stmt(if_statement),
            Statement::While(while_statement) => self.eval_while_stmt(while_statement),
            Statement::FunDecl(func_decl) => self.eval_fun_decl(func_decl),
            Statement::Return(return_statement) => self.eval_return_stmt(return_statement),
            Statement::ClassDecl(class_decl) => self.eval_class_decl(class_decl),
        }
    }

    fn eval_class_decl(&mut self, stmt: &'a ClassDecl) -> Result<'a, ()> {
        let name = stmt.name.lexeme.clone();

        let superclass = match &stmt.superclass {
            Some(expr @ Expr::Variable { name, .. }) => match self.eval_expr(expr)? {
                Value::Class(class) => Some(class),
                _ => return Err(RuntimeError::SuperclassMustBeAClass { token: name.clone() }),
            },
            None => None,
            Some(_) => unreachable!(),
        };

        self.environment.borrow_mut().define(name.clone(), Value::Nil);

        if let Some(superclass) = &superclass {
            self.environment = Environment::boxed_with_enclosing(&self.environment);
            self.environment
                .borrow_mut()
                .define("super".to_string(), Value::Class(superclass.clone()));
        }

        let methods = stmt
            .methods
            .iter()
            .map(|decl| {
                let method_name = decl.name.lexeme.clone();
                let closure = BoxedEnvironment::clone(&self.environment);
                let is_init = method_name == "init";
                (method_name, Rc::new(Function::new(decl, closure, is_init)))
            })
            .collect();

        let class = Class::new(name, methods, superclass.clone());

        if superclass.is_some() {
            let enclosing_env = self.environment.borrow().enclosing().unwrap();
            self.environment = enclosing_env;
        }
        self.environment
            .borrow_mut()
            .assign(stmt.name.clone(), Value::Class(Rc::new(class)))?;
        Ok(())
    }

    fn eval_var_decl(&mut self, stmt: &'a VariableDecl) -> Result<'a, ()> {
        let name = stmt.name.lexeme.clone();
        let value = match &stmt.initializer {
            Some(initializer) => self.eval_expr(initializer)?,
            None => Value::Nil,
        };
        self.environment.borrow_mut().define(name, value);
        Ok(())
    }

    fn eval_fun_decl(&mut self, stmt: &'a FunctionDecl) -> Result<'a, ()> {
        let function = Function::new(stmt, BoxedEnvironment::clone(&self.environment), false);
        self.environment.borrow_mut().define(stmt.name.lexeme.clone(), Value::Function(Rc::new(function)));
        Ok(())
    }

    fn eval_print_stmt(&mut self, stmt: &PrintStatement) -> Result<'a, ()> {
        let value = self.eval_expr(&stmt.expr)?;
        println!("{}", value);
        Ok(())
    }

    fn eval_return_stmt(&mut self, stmt: &ReturnStatement) -> Result<'a, ()> {
        let value = match &stmt.value {
            Some(value) => self.eval_expr(value)?,
            None => Value::Nil,
        };
        Err(RuntimeError::Return(Some(value)))
    }

    fn eval_block_stmt(&mut self, stmts: &'a [Statement], env: BoxedEnvironment<'a>) -> Result<'a, ()> {
        let old_env = BoxedEnvironment::clone(&self.environment);
        self.environment = env;
        for statement in stmts {
            match self.eval_stmt(statement) {
                Ok(()) => continue,
                err @ Err(_) => {
                    self.environment = old_env;
                    return err;
                },
            }
        }
        self.environment = old_env;
        Ok(())
    }

    fn eval_expr_stmt(&mut self, stmt: &ExpressionStatement) -> Result<'a, ()> {
        self.eval_expr(&stmt.expr)?;
        Ok(())
    }

    fn eval_if_stmt(&mut self, stmt: &'a IfStatemnet) -> Result<'a, ()> {
        let condition_result = self.eval_expr(&stmt.condition)?;
        if is_true(&condition_result) {
            self.eval_stmt(&stmt.if_branch)?;
        } else if let Some(stmt) = &stmt.else_branch {
            self.eval_stmt(&stmt)?;
        }
        Ok(())
    }

    fn eval_while_stmt(&mut self, stmt: &'a WhileStatement) -> Result<'a, ()> {
        while is_true(&self.eval_expr(&stmt.condition)?) {
            self.eval_stmt(&stmt.body)?;
        }
        Ok(())
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<'a, Value<'a>> {
        match expr {
            Expr::Asign { name, value, height } => self.eval_assignment(name, value, height),
            Expr::Binary { left, operator, right } => self.eval_binary(left, operator, right),
            Expr::Unary { operator, expr } => self.eval_unary(operator, expr),
            Expr::Grouping(expr) => self.eval_expr(expr),
            Expr::Literal(literal) => Ok(Value::from(literal)),
            Expr::Variable { name, height } => self.eval_variable(name, height),
            Expr::LogicalOr { left, right } => self.eval_or(left, right),
            Expr::LogicalAnd { left, right } => self.eval_and(left, right),
            Expr::Call { callee, paren, args } => self.eval_call(callee, paren, args),
            Expr::Get { object, name } => self.eval_get(object, name),
            Expr::Set { object, name, value } => self.eval_set(object, name, value),
            Expr::This { keyword, height } => self.eval_this(keyword, height),
            Expr::Super { keyword, method, height } => self.eval_super(keyword, method, height),
        }
    }

    fn eval_super(&mut self, keyword: &Token, method: &Token, height: &Cell<Option<usize>>) -> Result<'a, Value<'a>> {
        let Some(Value::Class(superclass)) = self.lookup_var(keyword, height.get()) else {
            panic!("Superclass not found");
        };
        let Value::Instance(object) = self.environment.borrow().get_at("this", height.get().unwrap() - 1).unwrap() else {
            panic!("This is not found");
        };
        let Some(method) = superclass.method(&method.lexeme) else {
            return Err(RuntimeError::UndefinedProperty { token: method.clone() });
        };
        let method = method.bind(&object);
        Ok(Value::Function(Rc::new(method)))
    }

    fn eval_assignment(&mut self, name: &Token, value: &Box<Expr>, height: &Cell<Option<usize>>) -> Result<'a, Value<'a>> {
        let value = self.eval_expr(value)?;
        match height.get() {
            Some(h) => self.environment.borrow_mut().assign_at(name.clone(), value.clone(), h),
            None => self.globals.borrow_mut().assign(name.clone(), value.clone())?,
        }
        Ok(value)
    }

    fn eval_variable(&mut self, name: &Token, height: &Cell<Option<usize>>) -> Result<'a, Value<'a>> {
        match self.lookup_var(name, height.get()) {
            Some(value) => Ok(value.clone()),
            None => Err(RuntimeError::UndefinedVariable { token: name.clone() }),
        }
    }

    fn eval_this(&mut self, keyword: &Token, height: &Cell<Option<usize>>) -> Result<'a, Value<'a>> {
        match self.lookup_var(keyword, height.get()) {
            Some(value) => Ok(value),
            None => Err(RuntimeError::UndefinedVariable { token: keyword.clone() }),
        }
    }

    fn eval_get(&mut self, object: &Expr, name: &Token) -> Result<'a, Value<'a>> {
        match self.eval_expr(object)? {
            Value::Instance(instance) => Instance::get(&instance, name),
            _ => Err(RuntimeError::NotAnInstance { token: name.clone() }),
        }
    }

    fn eval_set(&mut self, object: &Expr, name: &Token, value: &Expr) -> Result<'a, Value<'a>> {
        let Value::Instance(object) = self.eval_expr(object)? else {
            return Err(RuntimeError::NotAnInstance { token: name.clone() });
        };
        let value = self.eval_expr(value)?;
        object.borrow_mut().set(&name.lexeme, value.clone());
        Ok(value)
    }

    fn eval_or(&mut self, left: &Expr, right: &Expr) -> Result<'a, Value<'a>> {
        let left_value = self.eval_expr(left)?;
        if is_true(&left_value) {
            return Ok(left_value);
        } else {
            return Ok(self.eval_expr(right)?);
        }
    }

    fn eval_and(&mut self, left: &Expr, right: &Expr) -> Result<'a, Value<'a>> {
        let left_value = self.eval_expr(left)?;
        if !is_true(&left_value) {
            return Ok(left_value);
        } else {
            return Ok(self.eval_expr(right)?);
        }
    }

    fn eval_call(&mut self, callee: &Expr, paren: &Token, args: &[Expr]) -> Result<'a, Value<'a>> {

        let callee = self.eval_expr(callee)?;
        if !matches!(callee, Value::Function(_) | Value::NativeFunction(_) | Value::Class(_)) {
            return Err(RuntimeError::NotValidCallable { token: paren.clone() });
        }

        let arg_len = match &callee {
            Value::Function(func) => func.arity(),
            Value::NativeFunction(func) => func.arity(),
            Value::Class(class) => class.arity(),
            _ => unreachable!(),
        };

        if args.len() != arg_len {
            return Err(RuntimeError::InvalidArgumentCount {
                token: paren.clone(),
                expected: arg_len,
                actual: args.len(),
            });
        }

        let args = args.iter().map(|arg| self.eval_expr(arg)).collect::<Result<Vec<_>>>()?;
        match &callee {
            Value::Function(func) => func.call(self, args),
            Value::NativeFunction(native) => native.call(args),
            Value::Class(class) => Class::init(&class, self, args),
            _ => unreachable!(),
        }
    }

    fn eval_binary(&mut self, left: &Expr, operator: &Token, right: &Expr) -> Result<'a, Value<'a>> {
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
            (_, Plus | Minus | Div | Star | Greater | GreaterEq | Less | LessEq, _) => Err(RuntimeError::IncompatibleOperandType {
                operator: operator.clone(),
                message: "Operands must be numbers".to_string(),
            }),

            _ => panic!("Invalid binary operation"),
        }
    }

    fn eval_unary(&mut self, operator: &Token, expr: &Expr) -> Result<'a, Value<'a>> {
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

impl <'a> TreeWalk<'a> {
    fn lookup_var(&self, name: &Token, height: Option<usize>) -> Option<Value<'a>> {
        match height {
            Some(h) => self.environment.borrow().get_at(&name.lexeme, h),
            None => self.globals.borrow().get(&name.lexeme),
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
