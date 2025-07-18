use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::rc::Rc;

use crate::function::Function;
use crate::instance::Instance;
use crate::interpreter::{Interpreter, RuntimeError};
use crate::syntax::Value;

#[derive(Debug, Clone)]
pub struct Class<'a> {
    name: String,
    methods: HashMap<String, Rc<Function<'a>>>,
    superclass: Option<Rc<Class<'a>>>,
}

impl <'a> Class<'a> {
    pub fn new(name: String, methods: HashMap<String, Rc<Function<'a>>>, superclass: Option<Rc<Class<'a>>>) -> Self {
        Self { name, methods, superclass }
    }
}

impl <'a> Class <'a> {
    pub fn init(class: &Rc<Class<'a>>, interpreter: &mut impl Interpreter<'a>, args: Vec<Value<'a>>) -> Result<Value<'a>, RuntimeError<'a>> {
        let instance: Rc<RefCell<Instance>> = Instance::boxed(Rc::clone(class));
        if let Some(initializer) = class.method("init") {
            initializer.bind(&instance).call(interpreter, args)?;
        }
        Ok(Value::Instance(instance))
    }

    pub fn arity(&self) -> usize {
        self.method("init").map(|init| init.arity()).unwrap_or(0)
    }
}

impl <'a> Class<'a> {
    pub fn method(&self, name: &str) -> Option<Rc<Function<'a>>> {
        self.methods
            .get(name)
            .cloned()
            .or_else(|| self.superclass.as_ref().and_then(|superclass| superclass.method(name)))
    }
}

impl Display for Class<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl PartialEq for Class<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl PartialOrd for Class<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.name.cmp(&other.name))
    }
}
