use std::{fmt::Display, time::SystemTime};

use crate::{interpreter::RuntimeError, syntax::Value};

pub trait Callable {
    fn call(&self, arguments: Vec<Value>) -> anyhow::Result<Value, RuntimeError>;
    fn arity(&self) -> usize;
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum CallableVariant {
    Native(NativeFunction),
}


impl Callable for CallableVariant {
    fn call(&self, args: Vec<Value>) -> anyhow::Result<Value, RuntimeError> {
        match self {
            CallableVariant::Native(native) => native.call(args),
        }
    }

    fn arity(&self) -> usize {
        match self {
            CallableVariant::Native(native) => native.arity(),
        }
    }
}

impl Display for CallableVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallableVariant::Native(native) => write!(f, "{native}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
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
        Self {
            name: "clock",
            arity: 0,
            native: clock,
        }
    }
}

impl Callable for NativeFunction {
    fn call(&self, args: Vec<Value>) -> anyhow::Result<Value, RuntimeError> {
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