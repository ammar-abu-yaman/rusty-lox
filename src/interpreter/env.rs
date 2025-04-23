use std::collections::HashMap;

use crate::{syntax::Value, token::Token};

use super::{data::Variable, RuntimeError};


type GlobalVarStore = HashMap<String, Variable>;

pub struct Environment {
    values: HashMap<String, Variable>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            values: std::collections::HashMap::new(),
        }
    }

    pub fn define(&mut self, name: String, value: Variable) {
        self.values.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<&Variable> {
        self.values.get(name)
    }

    pub fn assign(&mut self, name: Token, value: Variable) -> Result<(), RuntimeError> {
        match self.values.get_mut(&name.lexeme) {
            Some(existing_value) => {
                *existing_value = value;
                Ok(())
            }
            None => Err(RuntimeError::UndefinedVariable { token: name }),
        }
    }
}