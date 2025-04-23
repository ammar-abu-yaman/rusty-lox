use std::{collections::HashMap, mem};

use crate::token::Token;

use super::{data::Variable, RuntimeError};

type BoxedEnvironment = Box<Environment>;
type GlobalVarStore = HashMap<String, Variable>;

pub struct Environment {
    values: GlobalVarStore,
    enclosing: Option<BoxedEnvironment>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            values: GlobalVarStore::new(),
            enclosing: None,
        }
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment {
    pub fn from_enclosing(enclosing: Environment) -> Self {
        Self {
            values: GlobalVarStore::new(),
            enclosing: Some(BoxedEnvironment::new(enclosing)),
        }
    }
}

impl Environment {
    pub fn define(&mut self, name: String, value: Variable) {
        self.values.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<&Variable> {
        match self.values.get(name) {
            Some(value) => Some(value),
            None => match &self.enclosing {
                Some(enclosing) => enclosing.get(name),
                None => None,
            },
        }
    }

    pub fn assign(&mut self, name: Token, value: Variable) -> Result<(), RuntimeError> {
        match self.values.get_mut(&name.lexeme) {
            Some(existing_value) => {
                *existing_value = value;
                Ok(())
            }
            None => match &mut self.enclosing {
                Some(enclosing) => enclosing.assign(name, value),
                None => Err(RuntimeError::UndefinedVariable { token: name }),
            },
        }
    }

    pub fn pop_env(&mut self) {
        if let Some(enclosing) = self.enclosing.take() {
            *self = *enclosing;
        }
    }

    pub fn push_env(&mut self) {
        let mut new_env = Environment::new();
        mem::swap(self, &mut new_env);
        self.enclosing = Some(BoxedEnvironment::new(new_env));
    }
}