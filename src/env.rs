use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{syntax::Value, token::Token};

use super::interpreter::RuntimeError;

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
    pub fn get(&self, name: &str) -> Option<Value> {
        match self.values.get(name) {
            Some(value) => Some(value.clone()),
            None => match &self.enclosing {
                Some(enclosing) => enclosing.borrow().get(name).clone(),
                None => None,
            },
        }
    }

    pub fn get_at(&self, name: &str, height: usize) -> Option<Value> {
        match height {
            0 => self.values.get(name).cloned(),
            h => self.enclosing.as_ref().and_then(|e| e.borrow().get_at(name, h - 1).clone())
        }
    }

    pub fn define(&mut self, name: impl Into<String>, value: Value) {
        self.values.insert(name.into(), value);
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

    pub fn assign_at(&mut self, name: Token, value: Value, height: usize) {
        match height {
            0 => { self.values.insert(name.lexeme.clone(), value); },
            h => { 
                self.enclosing
                    .as_ref()
                    .map(|e| e.borrow_mut().assign_at(name, value, h - 1));
            }
        };
    }
}