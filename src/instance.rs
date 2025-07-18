use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::rc::Rc;

use crate::class::Class;
use crate::interpreter::RuntimeError;
use crate::syntax::Value;
use crate::token::Token;

#[derive(Debug, Clone)]
pub struct Instance<'a> {
    class: Rc<Class<'a>>,
    fields: HashMap<String, Value<'a>>,
}

impl <'a> Instance<'a> {
    pub fn new(class: Rc<Class<'a>>) -> Self {
        Self { class, fields: HashMap::new() }
    }

    pub fn boxed(class: Rc<Class<'a>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new(class)))
    }
}

impl <'a> Instance<'a> {
    pub fn get(this: &Rc<RefCell<Self>>, name: &Token) -> Result<Value<'a>, RuntimeError<'a>> {
        if let Some(field) = this.borrow().fields.get(&name.lexeme) {
            return Ok(field).cloned();
        }
        if let Some(method) = this.borrow().class.method(&name.lexeme) {
            return Ok(Value::Function(Rc::new(method.bind(this))));
        }
        Err(RuntimeError::UndefinedProperty { token: name.clone() })
    }

    pub fn set(&mut self, name: impl Into<String>, value: Value<'a>) {
        self.fields.insert(name.into(), value);
    }
}

impl Display for Instance<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} instance", self.class)
    }
}

impl PartialEq for Instance<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.class == other.class
    }
}

impl PartialOrd for Instance<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.class.partial_cmp(&other.class)
    }
}
