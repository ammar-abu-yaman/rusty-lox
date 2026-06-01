use std::{fmt::Display, panic};

#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    String { value: String, next: *mut Object },
}

impl<T: Into<String>> From<T> for Object {
    fn from(value: T) -> Self {
        Self::String {
            value: value.into(),
            next: std::ptr::null_mut(),
        }
    }
}

impl Object {
    pub fn next(&self) -> Option<&mut Object> {
        use Object::*;
        match self {
            String { next, .. } => unsafe { next.as_mut() },
        }
    }

    pub fn set_next(&mut self, next_ptr: *mut Object) {
        use Object::*;
        match self {
            String { next, .. } => *next = next_ptr,
        }
    }

    pub fn str(&self) -> &str {
        use Object::*;
        match self {
            String { value, .. } => &value,
            _ => panic!("Expected a string, got {:?}", self),
        }
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Object::String { .. })
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Object::*;
        match self {
            String { value, .. } => write!(f, "{}", value),
        }
    }
}
