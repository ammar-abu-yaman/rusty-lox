use super::object::Object;
use std::{collections::HashMap, hash::Hash};

pub struct MemoryManager {
    objects: *mut Object,
    strings: HashMap<StrPtr, *mut Object>,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            objects: std::ptr::null_mut(),
            strings: HashMap::new(),
        }
    }
}

impl MemoryManager {
    pub fn intern_string(&mut self, string: &str) -> *mut Object {
        if let Some(ptr) = self.strings.get(&StrPtr(string as *const str)) {
            return *ptr;
        }
        let ptr = self.allocate_string(string);
        self.strings.insert(unsafe { StrPtr(ptr.as_ref_unchecked().str() as *const str) }, ptr);
        ptr
    }

    pub fn allocate_string(&mut self, string: impl Into<String>) -> *mut Object {
        let object = Object::from(string);
        let ptr = unsafe { self.allocate_object(object) };
        ptr
    }

    pub unsafe fn allocate_object(&mut self, mut object: Object) -> *mut Object {
        object.set_next(self.objects);
        let ptr = Box::into_raw(Box::new(object));
        self.objects = ptr;
        ptr
    }
}

struct StrPtr(*const str);

impl Hash for StrPtr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        unsafe {
            self.0.as_ref_unchecked().hash(state);
        }
    }
}

impl PartialEq for StrPtr {
    fn eq(&self, other: &Self) -> bool {
        unsafe { self.0.as_ref() == other.0.as_ref() }
    }
}

impl Eq for StrPtr {}
