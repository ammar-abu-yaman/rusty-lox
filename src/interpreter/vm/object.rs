use std::{fmt::Display, panic};

use crate::interpreter::vm::Chunk;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object {
    pub next: *mut Object,
    pub data: ObjectData,
}

#[derive(Debug, Clone)]
pub enum ObjectData {
    String(String),
    Function(ObjFunction),
}

#[derive(Debug, Clone)]
pub struct ObjFunction {
    pub arity: usize,
    pub name: *mut Object,
    pub chunk: Chunk,
}

impl Object {
    pub fn string(value: impl Into<String>) -> Self {
        Self {
            next: std::ptr::null_mut(),
            data: ObjectData::String(value.into()),
        }
    }

    pub fn function(arity: usize, name: *mut Object) -> Self {
        Self {
            next: std::ptr::null_mut(),
            data: ObjectData::Function(ObjFunction {
                arity,
                name,
                chunk: Chunk::default(),
            }),
        }
    }
}

impl Object {
    pub fn as_str(&self) -> &str {
        match &self.data {
            ObjectData::String(value) => &value,
            _ => panic!("Expected a string, got {:?}", self),
        }
    }

    pub fn as_function(&self) -> &ObjFunction {
        match &self.data {
            ObjectData::Function(fun) => fun,
            _ => panic!("Expected a function, got {:?}", self),
        }
    }

    pub fn is_string(&self) -> bool {
        matches!(self.data, ObjectData::String { .. })
    }

    pub fn is_function(&self) -> bool {
        matches!(self.data, ObjectData::Function { .. })
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.data {
            ObjectData::String(value) => write!(f, "{}", value),
            ObjectData::Function(fun) => unsafe { write!(f, "<fn {}>", fun.name.as_ref_unchecked().as_str()) },
        }
    }
}

impl PartialEq for ObjectData {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ObjectData::String(a), ObjectData::String(b)) => a == b,
            (ObjectData::Function(a), ObjectData::Function(b)) => a.name == b.name,
            _ => false,
        }
    }
}

impl Eq for ObjectData {}
