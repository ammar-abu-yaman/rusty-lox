use std::{cell::RefCell, collections::HashMap, fmt::Display, rc::Rc};

use crate::{class::Class, function::CallableVariant, interpreter::RuntimeError, syntax::Value, token::Token};

#[derive(Debug, Clone)]
pub struct Instance {
    class: Class,
    fields: HashMap<String, Value>,
}

impl Instance {
    pub fn new(class: Class) -> Self {
        Self { class, fields: HashMap::new() }
    }

    pub fn boxed(class: Class) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new(class)))
    }
}

impl Instance {
    pub fn get(this: &Rc<RefCell<Self>>, name: &Token) -> Result<Value, RuntimeError> {
        if let Some(field) = this.borrow().fields.get(&name.lexeme) {
            return Ok(field).cloned()
        }
        if let Some(method) = this.borrow().class.method(&name.lexeme) {
            return Ok(Value::Callable(CallableVariant::Defined(method.bind(this))));
        }
        Err(RuntimeError::UndefinedProperty { token: name.clone() })
    }

    pub fn set(&mut self, name: impl Into<String>, value: Value) {
        self.fields.insert(name.into(), value);
    }
}

impl Display for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} instance", self.class)
    }
}

impl PartialEq for Instance {
    fn eq(&self, other: &Self) -> bool {
        self.class == other.class
    }
}

impl PartialOrd for Instance {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.class.partial_cmp(&other.class)
    }
}