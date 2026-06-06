use super::object::Object;
use super::value::Value;
use std::{collections::HashMap, hash::Hash};

pub struct MemoryManager {
    objects: *mut Object,
    strings: HashMap<StrPtr, *mut Object>,
    globals: HashMap<StrPtr, Value>,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            objects: std::ptr::null_mut(),
            strings: HashMap::new(),
            globals: HashMap::new(),
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

    pub fn define_global(&mut self, key: *mut Object, value: Value) {
        let name = unsafe { StrPtr(key.as_ref_unchecked().str() as *const str) };
        self.globals.insert(name, value);
    }

    pub fn get_global(&self, key: *mut Object) -> Option<Value> {
        let name = unsafe { StrPtr(key.as_ref_unchecked().str() as *const str) };
        return self.globals.get(&name).copied();
    }

    pub fn set_global(&mut self, key: *mut Object, value: Value) -> bool {
        let name = unsafe { StrPtr(key.as_ref_unchecked().str() as *const str) };
        if !self.globals.contains_key(&name) {
            return false;
        }
        self.globals.insert(name, value);
        true
    }

    pub unsafe fn allocate_object(&mut self, mut object: Object) -> *mut Object {
        object.set_next(self.objects);
        let ptr = Box::into_raw(Box::new(object));
        self.objects = ptr;
        ptr
    }
}

impl Drop for MemoryManager {
    fn drop(&mut self) {
        self.globals.clear();
        self.strings.clear();
        let mut curr = self.objects;
        unsafe {
            while !curr.is_null() {
                let next = (*curr).next();
                drop(Box::from_raw(curr));
                curr = match next {
                    Some(obj) => obj,
                    None => std::ptr::null_mut(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_string() {
        let mut mm = MemoryManager::new();

        let p1 = mm.intern_string("hello");
        let p2 = mm.intern_string("hello");
        let p3 = mm.intern_string("world");

        assert_eq!(p1, p2, "Identical strings should return the same pointer");
        assert_ne!(p1, p3, "Different strings should return different pointers");

        // Test dynamic string
        let dynamic = String::from("hello");
        let p4 = mm.intern_string(&dynamic);
        assert_eq!(p1, p4, "Dynamic string with same content should be interned to same pointer");
    }

    #[test]
    fn test_allocate_string_always_new() {
        let mut mm = MemoryManager::new();

        let p1 = mm.allocate_string("hello");
        let p2 = mm.allocate_string("hello");

        assert_ne!(p1, p2, "allocate_string should always return a new pointer");
    }

    #[test]
    fn test_object_linked_list() {
        let mut mm = MemoryManager::new();

        let p1 = mm.allocate_string("a");
        let p2 = mm.allocate_string("b");
        let p3 = mm.allocate_string("c");

        assert_eq!(mm.objects, p3);
        unsafe {
            assert_eq!((*p3).next().map(|o| o as *mut Object), Some(p2));
            assert_eq!((*p2).next().map(|o| o as *mut Object), Some(p1));
            assert_eq!((*p1).next().map(|o| o as *mut Object), None);
        }
    }

    #[test]
    fn test_globals() {
        let mut mm = MemoryManager::new();

        let key = mm.intern_string("my_global");
        let val = Value::Number(42.0);

        mm.define_global(key, val);
        assert_eq!(mm.get_global(key), Some(val));
    }

    #[test]
    fn test_set_global_bug_confirmation() {
        let mut mm = MemoryManager::new();

        let key = mm.intern_string("a");
        let val1 = Value::Number(1.0);
        let val2 = Value::Number(2.0);

        // Current bug: returns false if NOT found, but it should return false if NOT found
        // Wait, the report said: "returns false if the variable already exists"
        // Let's check the code:
        // if self.globals.contains_key(&name) { return false; }
        // This means if it DOES exist, it returns false.
        // In Lox, set_global should fail (return false) if it DOES NOT exist.

        mm.define_global(key, val1);
        let result = mm.set_global(key, val2);

        // This will currently return false because it already exists
        assert_eq!(result, false, "Current implementation incorrectly fails if variable exists");
        assert_eq!(mm.get_global(key), Some(val1), "Value should not have changed due to bug");
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
