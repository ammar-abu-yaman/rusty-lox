use std::{collections::HashMap, fmt::Display};

use crate::{function::{Callable, Function}, instance::Instance, interpreter::{self, Interpreter, RuntimeError}, syntax::Value};

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
    fn call(&self, interpreter: &mut impl Interpreter, args: Vec<Value>) -> Result<Value, RuntimeError> {
        let instance = Instance::boxed(self.clone());
        if let Some(initializer) = self.method("init") {
            initializer.bind(&instance).call(interpreter, args)?;
        }
        Ok(Value::Instance(instance))
    }

    fn arity(&self) -> usize {
        self.method("init")
            .map(|init| init.arity())
            .unwrap_or(0)
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