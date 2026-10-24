use super::mem::StrPtr;
use super::{Object, Value};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ObjClass {
    pub(super) name: *mut Object,
    pub(super) methods: HashMap<StrPtr, Value>,
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

#[derive(Debug, Clone)]
pub struct ObjBoundedMethod {
    pub(super) reciever: Value,
    pub(super) closure: *mut Object,
}

impl ObjBoundedMethod {
    #[inline(always)]
    pub fn closure(&self) -> &super::object::ObjClosure {
        unsafe { self.closure.as_ref_unchecked().as_closure() }
    }

    #[inline(always)]
    pub fn closure_mut(&mut self) -> &mut super::object::ObjClosure {
        unsafe { self.closure.as_mut_unchecked().as_closure_mut() }
    }
}
