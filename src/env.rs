use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::interpreter::RuntimeError;
use crate::syntax::Value;
use crate::token::Token;

pub type BoxedEnvironment<'a, 't> = Rc<RefCell<Environment<'a, 't>>>;
pub type ValueMap<'a, 't> = HashMap<String, Value<'a, 't>>;

#[derive(Debug, Clone)]
pub struct Environment<'a, 't> {
    values: ValueMap<'a, 't>,
    pub enclosing: Option<BoxedEnvironment<'a, 't>>,
}

impl<'a, 't> Environment<'a, 't> {
    pub fn new() -> Self {
        Self {
            values: ValueMap::new(),
            enclosing: None,
        }
    }

    pub fn boxed() -> BoxedEnvironment<'a, 't> {
        BoxedEnvironment::new(RefCell::new(Self::new()))
    }

    pub fn boxed_with_enclosing(enclosing: &BoxedEnvironment<'a, 't>) -> BoxedEnvironment<'a, 't> {
        BoxedEnvironment::new(RefCell::new(Self {
            values: ValueMap::new(),
            enclosing: Some(enclosing.clone()),
        }))
    }
}

impl Default for Environment<'_, '_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, 't> Environment<'a, 't> {
    pub fn enclosing(&self) -> Option<BoxedEnvironment<'a, 't>> {
        self.enclosing.clone()
    }

    pub fn get(&self, name: &str) -> Option<Value<'a, 't>> {
        match self.values.get(name) {
            Some(value) => Some(value.clone()),
            None => match &self.enclosing {
                Some(enclosing) => enclosing.borrow().get(name).clone(),
                None => None,
            },
        }
    }

    pub fn get_at(&self, name: &str, height: usize) -> Option<Value<'a, 't>> {
        match height {
            0 => self.values.get(name).cloned(),
            h => self.enclosing.as_ref().and_then(|e| e.borrow().get_at(name, h - 1).clone()),
        }
    }

    pub fn define(&mut self, name: impl Into<String>, value: Value<'a, 't>) {
        self.values.insert(name.into(), value);
    }

    pub fn assign(&mut self, name: Token<'t>, value: Value<'a, 't>) -> Result<(), RuntimeError<'a, 't>> {
        match self.values.get_mut(name.lexeme) {
            Some(existing_value) => {
                *existing_value = value;
                Ok(())
            },
            None => match &mut self.enclosing {
                Some(enclosing) => enclosing.borrow_mut().assign(name, value),
                None => Err(RuntimeError::UndefinedVariable { token: name }),
            },
        }
    }

    pub fn assign_at(&mut self, name: Token<'t>, value: Value<'a, 't>, height: usize) {
        match height {
            0 => {
                self.values.insert(name.lexeme.to_string(), value);
            },
            h => {
                self.enclosing.as_ref().map(|e| e.borrow_mut().assign_at(name, value, h - 1));
            },
        };
    }
}
