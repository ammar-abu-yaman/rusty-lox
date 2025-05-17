use std::{cell::RefCell, fmt::{Debug, Display}, rc::Rc, time::SystemTime};

use crate::{class::Class, instance::Instance, interpreter::{BoxedEnvironment, Environment, Interpreter, RuntimeError}, syntax::{BlockStatement, FunctionDecl, Statement, Value}, token::Token};

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

pub trait Callable {
    fn call(&self, interpreter: &mut impl Interpreter, arguments: Vec<Value>) -> anyhow::Result<Value, RuntimeError>;
    fn arity(&self) -> usize;
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum CallableVariant {
    Native(NativeFunction),
    Defined(Function),
    Class(Class),
}


impl Callable for CallableVariant {
    fn call(&self, interpreter: &mut impl Interpreter, args: Vec<Value>) -> anyhow::Result<Value, RuntimeError> {
        match self {
            CallableVariant::Native(native) => native.call(interpreter, args),
            CallableVariant::Defined(function) => function.call(interpreter, args),
            CallableVariant::Class(class) => class.call(interpreter, args),
        }
    }

    fn arity(&self) -> usize {
        match self {
            CallableVariant::Native(native) => native.arity(),
            CallableVariant::Defined(function) => function.arity(),
            CallableVariant::Class(class) => class.arity(),
        }
    }
}

impl Display for CallableVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallableVariant::Native(native) => write!(f, "{native}"),
            CallableVariant::Defined(function) => write!(f, "{function}"),
            CallableVariant::Class(class) => write!(f, "{class}"),
        }
    }
}


#[derive(Clone)]
pub struct Function {
    name: Token,
    params: Vec<Token>,
    body: Vec<Statement>,
    closure: BoxedEnvironment,
}

impl Function {
    pub fn new(decl: &FunctionDecl, env: BoxedEnvironment) -> Self {
        Self { 
            name: decl.name.clone(),
            params: decl.params.clone(),
            body: decl.body.clone(),
            closure: env,
        }
    }
}

impl Function {
    pub fn bind(&self, instance: &Rc<RefCell<Instance>>) -> Self {
        let binded_env = Environment::boxed_with_enclosing(&self.closure);
        binded_env.borrow_mut().define("this", Value::Instance(Rc::clone(instance)));
        Self {
            name: self.name.clone(),
            params: self.params.clone(),
            body: self.body.clone(),
            closure: binded_env,
        }
    }
}

impl Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<fn {}>", self.name.lexeme)
    }
}

impl Callable for Function {
    fn call(&self, interpreter: &mut impl Interpreter, args: Vec<Value>) -> anyhow::Result<Value, RuntimeError> {
        let environment = Environment::boxed_with_enclosing(&self.closure);
        let mut args = args.into_iter();
        for param in &self.params {
            environment.borrow_mut().define(param.lexeme.clone(), args.next().unwrap());
        }
        match interpreter.interpret_block(&BlockStatement{ statements: self.body.clone() } , environment) {
            Ok(_) => Ok(Value::Nil),
            Err(RuntimeError::Return(value)) => Ok(value.unwrap_or(Value::Nil)),
            Err(e) => Err(e),
        }
    }

    fn arity(&self) -> usize {
        self.params.len()
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self.name.lexeme == other.name.lexeme
    }
}
impl PartialOrd for Function {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.name.lexeme.cmp(&other.name.lexeme))
    }
}

impl Display for Function {
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

impl Callable for NativeFunction {
    fn call(&self, _interpreter: &mut impl Interpreter, args: Vec<Value>) -> anyhow::Result<Value, RuntimeError> {
        (self.native)(args)
    }

    fn arity(&self) -> usize {
        self.arity
    }
}

impl Display for NativeFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<native fn>")
    }
}

fn clock(_args: Vec<Value>) -> anyhow::Result<Value, RuntimeError> {
    let millis = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    Ok(Value::Number(millis))
}