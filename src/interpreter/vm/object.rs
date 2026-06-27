use std::{fmt::Display, panic};

use super::value::Value;
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
    NativeFunction(NativeFunction),
    Closure(ObjClosure),
    Upvalue(ObjUpvalue),
}

#[derive(Debug, Clone)]
pub struct ObjFunction {
    pub arity: usize,
    pub name: *mut Object,
    pub chunk: Chunk,
    pub upvalue_count: usize,
}

pub type NativeFunction = fn(&[Value]) -> Value;

#[derive(Debug, Clone)]
pub struct ObjClosure {
    pub function: *mut Object,
    pub upvalues: Vec<*mut Object>,
}

#[derive(Debug, Clone, Copy)]
pub struct ObjUpvalue {
    pub location: *mut Value,
    pub next: *mut Object,
    pub closed: Option<Value>,
}

impl ObjClosure {
    #[inline(always)]
    pub fn function(&self) -> &ObjFunction {
        return unsafe { self.function.as_ref_unchecked().as_function() };
    }

    #[inline(always)]
    pub fn function_mut(&mut self) -> &mut ObjFunction {
        return unsafe { self.function.as_mut_unchecked().as_function_mut() };
    }
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
                upvalue_count: 0,
            }),
        }
    }

    pub fn native(native_fn: NativeFunction) -> Self {
        Self {
            next: std::ptr::null_mut(),
            data: ObjectData::NativeFunction(native_fn),
        }
    }

    pub fn closure(function: *mut Object, upvalue_count: usize) -> Self {
        Self {
            next: std::ptr::null_mut(),
            data: ObjectData::Closure(ObjClosure {
                function,
                upvalues: vec![std::ptr::null_mut(); upvalue_count],
            }),
        }
    }

    pub fn upvalue(value: *mut Value) -> Self {
        Self {
            next: std::ptr::null_mut(),
            data: ObjectData::Upvalue(ObjUpvalue {
                location: value,
                next: std::ptr::null_mut(),
                closed: None,
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

    pub fn as_closure(&self) -> &ObjClosure {
        match &self.data {
            ObjectData::Closure(closure) => closure,
            _ => panic!("Expected a closure, got {:?}", self),
        }
    }

    pub fn as_closure_mut(&mut self) -> &mut ObjClosure {
        if !matches!(self.data, ObjectData::Closure { .. }) {
            panic!("Expected a closure, got {:?}", self);
        }
        match &mut self.data {
            ObjectData::Closure(closure) => closure,
            _ => unreachable!(),
        }
    }

    pub fn as_function_mut(&mut self) -> &mut ObjFunction {
        if !matches!(self.data, ObjectData::Function { .. }) {
            panic!("Expected a function, got {:?}", self);
        }
        match &mut self.data {
            ObjectData::Function(fun) => fun,
            _ => unreachable!(),
        }
    }

    pub fn as_upvalue(&self) -> &ObjUpvalue {
        match &self.data {
            ObjectData::Upvalue(upvalue) => upvalue,
            _ => panic!("Expected an upvalue, got {:?}", self),
        }
    }

    pub fn as_upvalue_mut(&mut self) -> &mut ObjUpvalue {
        if !matches!(self.data, ObjectData::Upvalue { .. }) {
            panic!("Expected an upvalue, got {:?}", self);
        }
        match &mut self.data {
            ObjectData::Upvalue(upvalue) => upvalue,
            _ => unreachable!(),
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
            ObjectData::Function(fun) => unsafe {
                if let Some(name) = fun.name.as_ref() {
                    write!(f, "<fn {}>", name.as_str())
                } else {
                    write!(f, "<script>")
                }
            },
            ObjectData::NativeFunction(_) => write!(f, "<native fn>"),
            ObjectData::Closure(ObjClosure { function, .. }) => unsafe { function.as_ref_unchecked().fmt(f) },
            ObjectData::Upvalue(_) => write!(f, "upvalue"),
        }
    }
}

impl PartialEq for ObjectData {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ObjectData::String(a), ObjectData::String(b)) => a == b,
            (ObjectData::Function(a), ObjectData::Function(b)) => a.name == b.name,
            (ObjectData::NativeFunction(a), ObjectData::NativeFunction(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for ObjectData {}

pub fn native_clock(_args: &[Value]) -> Value {
    return Value::Number(std::time::Instant::now().elapsed().as_secs_f64());
}
