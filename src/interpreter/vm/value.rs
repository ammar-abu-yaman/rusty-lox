use std::fmt::Display;

use super::object::Object;

#[derive(Debug, Clone, Copy)]
pub enum Value {
    Number(f64),
    Bool(bool),
    Object(*mut Object),
    Nil,
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{n}"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Nil => write!(f, "nil"),
            Self::Object(o) => {
                let o = unsafe { o.as_ref_unchecked() };
                write!(f, "{}", o)
            },
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Object(a), Value::Object(b)) => a == b || unsafe { (**a).data == (**b).data },
            _ => false,
        }
    }
}

impl Value {
    pub fn as_number(&self) -> f64 {
        match self {
            Self::Number(n) => *n,
            _ => unreachable!("expected number, got {self}"),
        }
    }

    pub fn as_object(&self) -> &Object {
        match self {
            Self::Object(o) => unsafe { o.as_ref_unchecked() },
            _ => unreachable!("expected object, got {self}"),
        }
    }

    pub fn as_object_ptr(&self) -> *mut Object {
        match self {
            Self::Object(o) => *o,
            _ => unreachable!("expected object, got {self}"),
        }
    }

    pub fn as_mut_object(&mut self) -> &mut Object {
        match self {
            Self::Object(o) => unsafe { o.as_mut_unchecked() },
            _ => unreachable!("expected object, got {self}"),
        }
    }

    pub fn is_falsy(&self) -> bool {
        matches!(self, Self::Nil | Self::Bool(false))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }
    pub fn is_string_object(&self) -> bool {
        match self {
            Self::Object(ptr) => unsafe { ptr.as_ref_unchecked().is_string() },
            _ => false,
        }
    }

    pub fn mark(&mut self, gray_stack: &mut Vec<*mut Object>) {
        if let Self::Object(ptr) = self {
            ptr.mark_ptr(gray_stack);
        }
    }
}
