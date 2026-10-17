use super::mem::StrPtr;
use super::{Object, Value};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ObjClass {
    pub(super) name: *mut Object,
}

#[derive(Debug, Clone)]
pub struct ObjInstance {
    pub(super) class: *mut Object,
    pub(super) fields: HashMap<StrPtr, Value>,
}

impl ObjInstance {
    pub fn get_property(&self, name: &str) -> Option<Value> {
        self.fields.get(&StrPtr(std::ptr::from_ref(name))).cloned()
    }

    pub fn set_property(&mut self, name: &str, value: Value) {
        self.fields.insert(StrPtr(std::ptr::from_ref(name)), value);
    }
}
