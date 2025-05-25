use std::collections::HashMap;
use std::fmt::Display;
use std::rc::Rc;

use crate::function::{Callable, Function};
use crate::instance::Instance;
use crate::interpreter::{Interpreter, RuntimeError};
use crate::syntax::Value;

#[derive(Debug, Clone)]
pub struct Class {
    name: String,
    methods: HashMap<String, Function>,
    superclass: Option<Rc<Class>>,
}

impl Class {
    pub fn new(name: String, methods: HashMap<String, Function>, superclass: Option<Rc<Class>>) -> Self {
        Self { name, methods, superclass }
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
        self.method("init").map(|init| init.arity()).unwrap_or(0)
    }
}

impl Class {
    pub fn method(&self, name: &str) -> Option<Function> {
        self.methods
            .get(name)
            .cloned()
            .or_else(|| self.superclass.as_ref().and_then(|superclass| superclass.method(name)))
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
