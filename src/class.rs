use std::fmt::Display;

use crate::{function::Callable, instance::Instance, interpreter::{Interpreter, RuntimeError}, syntax::Value};

#[derive(Debug, Clone)]
pub struct Class {
    name: String,
}

impl Class {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl Callable for Class {
    fn call(&self, _interpreter: &mut impl Interpreter, _arguments: Vec<Value>) -> Result<Value, RuntimeError> {
        Ok(Value::Instance(Instance::new(self.clone())))
    }

    fn arity(&self) -> usize {
        0
    }
}

impl Display for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl PartialEq for Class {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl PartialOrd for Class {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.name.cmp(&other.name))
    }
}