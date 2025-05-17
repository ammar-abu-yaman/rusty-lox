use std::{collections::HashMap, fmt::Display};

use crate::{function::{Callable, Function}, instance::Instance, interpreter::{Interpreter, RuntimeError}, syntax::Value};

#[derive(Debug, Clone)]
pub struct Class {
    name: String,
    methods: HashMap<String, Function>,
}

impl Class {
    pub fn new(name: String, methods: HashMap<String, Function>) -> Self {
        Self { name, methods }
    }
}

impl Callable for Class {
    fn call(&self, _interpreter: &mut impl Interpreter, _arguments: Vec<Value>) -> Result<Value, RuntimeError> {
        Ok(Value::Instance(Instance::boxed(self.clone())))
    }

    fn arity(&self) -> usize {
        0
    }
}

impl Class {
    pub fn method(&self, name: &str) -> Option<Function> {
        self.methods.get(name).cloned()
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