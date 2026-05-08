use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::rc::Rc;

use crate::class::Class;
use crate::interpreter::RuntimeError;
use crate::syntax::Value;
use crate::token::Token;

#[derive(Debug, Clone)]
pub struct Instance<'a, 't> {
    class: Rc<Class<'a, 't>>,
    fields: HashMap<&'t str, Value<'a, 't>>,
}

impl<'a, 't> Instance<'a, 't> {
    pub fn new(class: Rc<Class<'a, 't>>) -> Self {
        Self { class, fields: HashMap::new() }
    }

    pub fn boxed(class: Rc<Class<'a, 't>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new(class)))
    }
}

impl<'a, 't> Instance<'a, 't> {
    pub fn get(this: &Rc<RefCell<Self>>, name: &Token<'t>) -> Result<Value<'a, 't>, RuntimeError<'a, 't>> {
        if let Some(field) = this.borrow().fields.get(name.lexeme) {
            return Ok(field).cloned();
        }
        if let Some(method) = this.borrow().class.method(&name.lexeme) {
            return Ok(Value::Function(Rc::new(method.bind(this))));
        }
        Err(RuntimeError::UndefinedProperty { token: name.clone() })
    }

    pub fn set(&mut self, name: &'t str, value: Value<'a, 't>) {
        self.fields.insert(name, value);
    }
}

impl Display for Instance<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} instance", self.class)
    }
}

impl PartialEq for Instance<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        self.class == other.class
    }
}

impl PartialOrd for Instance<'_, '_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.class.partial_cmp(&other.class)
    }
}
