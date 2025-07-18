use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::interpreter::RuntimeError;
use crate::syntax::Value;
use crate::token::Token;

pub type BoxedEnvironment<'a> = Rc<RefCell<Environment<'a>>>;
pub type ValueMap<'a> = HashMap<String, Value<'a>>;

#[derive(Debug, Clone)]
pub struct Environment<'a> {
    values: ValueMap<'a>,
    pub enclosing: Option<BoxedEnvironment<'a>>,
}

impl <'a> Environment<'a> {
    pub fn new() -> Self {
        Self {
            values: ValueMap::new(),
            enclosing: None,
        }
    }

    pub fn boxed() -> BoxedEnvironment<'a> {
        BoxedEnvironment::new(RefCell::new(Self::new()))
    }

    pub fn boxed_with_enclosing(enclosing: &BoxedEnvironment<'a>) -> BoxedEnvironment<'a> {
        BoxedEnvironment::new(RefCell::new(Self {
            values: ValueMap::new(),
            enclosing: Some(enclosing.clone()),
        }))
    }
}

impl Default for Environment<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl <'a> Environment<'a> {
    pub fn enclosing(&self) -> Option<BoxedEnvironment<'a>> {
        self.enclosing.clone()
    }

    pub fn get(&self, name: &str) -> Option<Value<'a>> {
        match self.values.get(name) {
            Some(value) => Some(value.clone()),
            None => match &self.enclosing {
                Some(enclosing) => enclosing.borrow().get(name).clone(),
                None => None,
            },
        }
    }

    pub fn get_at(&self, name: &str, height: usize) -> Option<Value<'a>> {
        match height {
            0 => self.values.get(name).cloned(),
            h => self.enclosing.as_ref().and_then(|e| e.borrow().get_at(name, h - 1).clone()),
        }
    }

    pub fn define(&mut self, name: impl Into<String>, value: Value<'a>) {
        self.values.insert(name.into(), value);
    }

    pub fn assign(&mut self, name: Token, value: Value<'a>) -> Result<(), RuntimeError<'a>> {
        match self.values.get_mut(&name.lexeme) {
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

    pub fn assign_at(&mut self, name: Token, value: Value<'a>, height: usize) {
        match height {
            0 => {
                self.values.insert(name.lexeme.clone(), value);
            },
            h => {
                self.enclosing.as_ref().map(|e| e.borrow_mut().assign_at(name, value, h - 1));
            },
        };
    }
}
