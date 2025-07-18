use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::rc::Rc;
use std::time::SystemTime;

use crate::instance::Instance;
use crate::interpreter::{BoxedEnvironment, Environment, Interpreter, RuntimeError};
use crate::syntax::{FunctionDecl, Statement, Value};
use crate::token::Token;

pub enum FunctionType {
    Function,
    Method,
}

impl Display for FunctionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionType::Function => write!(f, "function"),
            FunctionType::Method => write!(f, "method"),
        }
    }
}

#[derive(Clone)]
pub struct Function<'a> {
    name: Token,
    params: Vec<Token>,
    body: &'a [Statement],
    closure: BoxedEnvironment<'a>,
    is_init: bool,
}

impl <'a> Function<'a> {
    pub fn new(decl: &'a FunctionDecl, env: BoxedEnvironment<'a>, is_init: bool) -> Self {
        Self {
            name: decl.name.clone(),
            params: decl.params.clone(),
            body: &decl.body,
            closure: env,
            is_init,
        }
    }
}

impl <'a> Function<'a> {
    pub fn bind(&self, instance: &Rc<RefCell<Instance<'a>>>) -> Self {
        let binded_env = Environment::boxed_with_enclosing(&self.closure);
        binded_env.borrow_mut().define("this", Value::Instance(Rc::clone(instance)));
        Self {
            name: self.name.clone(),
            params: self.params.clone(),
            body: self.body.clone(),
            is_init: self.is_init,
            closure: binded_env,
        }
    }
}

impl <'a> Debug for Function<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<fn {}>", self.name.lexeme)
    }
}

impl <'a> Function<'a> {
    pub fn call(&self, interpreter: &mut impl Interpreter<'a>, args: Vec<Value<'a>>) -> anyhow::Result<Value<'a>, RuntimeError<'a>> {
        let environment = Environment::boxed_with_enclosing(&self.closure);
        let mut args = args.into_iter();
        for param in &self.params {
            environment.borrow_mut().define(param.lexeme.clone(), args.next().unwrap());
        }
        match interpreter.interpret_block(self.body, environment) {
            Ok(_) if self.is_init => Ok(self.closure.borrow().get("this").unwrap()),
            Ok(_) => Ok(Value::Nil),
            Err(RuntimeError::Return(_)) if self.is_init => Ok(self.closure.borrow().get("this").unwrap()),
            Err(RuntimeError::Return(value)) => Ok(value.unwrap_or(Value::Nil)),
            Err(e) => Err(e),
        }
    }

    pub fn arity(&self) -> usize {
        self.params.len()
    }
}

impl <'a> PartialEq for Function<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.name.lexeme == other.name.lexeme
    }
}
impl <'a> PartialOrd for Function<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.name.lexeme.cmp(&other.name.lexeme))
    }
}

impl <'a> Display for Function<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<fn {}>", self.name.lexeme)
    }
}

#[derive(Debug, Clone)]
pub struct NativeFunction {
    pub name: &'static str,
    pub arity: usize,
    native: fn(Vec<Value>) -> anyhow::Result<Value, RuntimeError>,
}

impl NativeFunction {
    pub fn new(name: &'static str, arity: usize, native: fn(Vec<Value>) -> anyhow::Result<Value, RuntimeError>) -> Self {
        Self { name, arity, native }
    }

    pub fn clock() -> Self {
        return Self::new("clock", 0, clock);
    }
}

impl PartialEq for NativeFunction {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl PartialOrd for NativeFunction {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.name.cmp(other.name))
    }
}

impl <'a> NativeFunction {
    pub fn call(&self, args: Vec<Value<'a>>) -> anyhow::Result<Value<'a>, RuntimeError<'a>> {
        (self.native)(args)
    }

    pub fn arity(&self) -> usize {
        self.arity
    }
}

impl Display for NativeFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<native fn>")
    }
}

fn clock(_args: Vec<Value>) -> anyhow::Result<Value, RuntimeError> {
    let millis = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs_f64();
    Ok(Value::Number(millis))
}
