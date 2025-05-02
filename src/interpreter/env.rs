use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{syntax::Value, token::Token};

use super::RuntimeError;

pub type BoxedEnvironment = Rc<RefCell<Environment>>;
pub type ValueMap = HashMap<String, Value>;

#[derive(Debug, Clone)]
pub struct Environment {
    values: ValueMap,
    pub enclosing: Option<BoxedEnvironment>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            values: ValueMap::new(),
            enclosing: None,
        }
    }

    pub fn boxed() -> BoxedEnvironment {
        BoxedEnvironment::new(RefCell::new(Self::new()))
    }

    pub fn boxed_with_enclosing(enclosing: &BoxedEnvironment) -> BoxedEnvironment {
        BoxedEnvironment::new(RefCell::new(Self {
            values: ValueMap::new(),
            enclosing: Some(BoxedEnvironment::clone(enclosing)),
        }))
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}


impl Environment {
    pub fn define(&mut self, name: impl Into<String>, value: Value) {
        self.values.insert(name.into(), value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        match self.values.get(name) {
            Some(value) => Some(value.clone()),
            None => match &self.enclosing {
                Some(enclosing) => enclosing.borrow_mut().get(name).clone(),
                None => None,
            },
        }
    }

    pub fn assign(&mut self, name: Token, value: Value) -> Result<(), RuntimeError> {
        match self.values.get_mut(&name.lexeme) {
            Some(existing_value) => {
                *existing_value = value;
                Ok(())
            }
            None => match &mut self.enclosing {
                Some(enclosing) => enclosing.borrow_mut().assign(name, value),
                None => Err(RuntimeError::UndefinedVariable { token: name }),
            },
        }
    }
}