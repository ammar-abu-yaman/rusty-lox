use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::rc::Rc;

use crate::function::Function;
use crate::instance::Instance;
use crate::interpreter::{Interpreter, RuntimeError};
use crate::syntax::Value;

#[derive(Debug, Clone)]
pub struct Class<'a, 't> {
    name: &'t str,
    methods: HashMap<&'t str, Rc<Function<'a, 't>>>,
    superclass: Option<Rc<Class<'a, 't>>>,
}

impl<'a, 't> Class<'a, 't> {
    pub fn new(name: &'t str, methods: HashMap<&'t str, Rc<Function<'a, 't>>>, superclass: Option<Rc<Class<'a, 't>>>) -> Self {
        Self { name, methods, superclass }
    }
}

impl<'a, 't> Class<'a, 't> {
    pub fn init(
        class: &Rc<Class<'a, 't>>,
        interpreter: &mut impl Interpreter<'a, 't>,
        args: Vec<Value<'a, 't>>,
    ) -> Result<Value<'a, 't>, RuntimeError<'a, 't>> {
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

impl<'a, 't> Class<'a, 't> {
    pub fn method(&self, name: &str) -> Option<Rc<Function<'a, 't>>> {
        self.methods
            .get(name)
            .cloned()
            .or_else(|| self.superclass.as_ref().and_then(|superclass| superclass.method(name)))
    }
}

impl Display for Class<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl PartialEq for Class<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl PartialOrd for Class<'_, '_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.name.cmp(&other.name))
    }
}
