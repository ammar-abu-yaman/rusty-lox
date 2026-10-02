use crate::interpreter::vm::object::{self, NativeFunction};

use super::object::Object;
use super::value::Value;
use std::{collections::HashMap, hash::Hash};

pub const GC_GROWTH_FACTOR: usize = 2;

pub struct MemoryManager {
    objects: *mut Object,
    strings: HashMap<StrPtr, *mut Object>,
    globals: HashMap<StrPtr, Value>,
    pub open_upvalues: *mut Object,
    pub bytes_allocated: usize,
    pub next_gc: usize,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            objects: std::ptr::null_mut(),
            strings: HashMap::new(),
            globals: HashMap::new(),
            open_upvalues: std::ptr::null_mut(),
            bytes_allocated: 0,
            next_gc: 1024usize * 1024usize,
        }
    }
}

impl MemoryManager {
    pub fn intern_string(&mut self, string: &str) -> *mut Object {
        if let Some(ptr) = self.strings.get(&StrPtr(string as *const str)) {
            return *ptr;
        }
        let ptr = self.allocate_string(string);
        self.strings.insert(unsafe { StrPtr(ptr.as_ref_unchecked().as_str() as *const str) }, ptr);
        ptr
    }

    pub fn allocate_string(&mut self, string: impl Into<String>) -> *mut Object {
        let object = Object::string(string);
        let ptr = unsafe { self.allocate_object(object) };
        ptr
    }

    pub fn allocate_function(&mut self, name: *mut Object, arity: usize) -> *mut Object {
        let object = Object::function(arity, name);
        let ptr = unsafe { self.allocate_object(object) };
        ptr
    }

    pub fn allocate_closure(&mut self, function: *mut Object, upvalue_count: usize) -> *mut Object {
        let object = Object::closure(function, upvalue_count);
        let ptr = unsafe { self.allocate_object(object) };
        ptr
    }

    pub fn allocate_upvalue(&mut self, value: *mut Value) -> *mut Object {
        let object = Object::upvalue(value);
        let ptr = unsafe { self.allocate_object(object) };
        ptr
    }

    pub fn allocate_native_function(&mut self, native_fn: NativeFunction) -> *mut Object {
        return unsafe { self.allocate_object(Object::native(native_fn)) };
    }

    pub fn define_global(&mut self, key: *mut Object, value: Value) {
        let name = unsafe { StrPtr(key.as_ref_unchecked().as_str() as *const str) };
        self.globals.insert(name, value);
    }

    pub fn get_global(&self, key: *mut Object) -> Option<Value> {
        let name = unsafe { StrPtr(key.as_ref_unchecked().as_str() as *const str) };
        return self.globals.get(&name).copied();
    }

    pub fn set_global(&mut self, key: *mut Object, value: Value) -> bool {
        let name = unsafe { StrPtr(key.as_ref_unchecked().as_str() as *const str) };
        if !self.globals.contains_key(&name) {
            return false;
        }
        self.globals.insert(name, value);
        true
    }

    pub unsafe fn allocate_object(&mut self, mut object: Object) -> *mut Object {
        self.bytes_allocated += object.get_size();
        object.next = self.objects;
        let ptr = Box::into_raw(Box::new(object));
        self.objects = ptr;
        #[cfg(feature = "debug-log-gc")]
        println!("{:p} allocate for {:?}", ptr, (*ptr).data);

        ptr
    }

    pub fn adjust_next_gc(&mut self) {
        self.next_gc = self.bytes_allocated * GC_GROWTH_FACTOR;
    }
}

impl MemoryManager {
    pub fn mark_globals(&mut self, gray_stack: &mut Vec<*mut Object>) {
        self.globals.iter_mut().for_each(|(key, value)| {
            if let Some(ptr) = self.strings.get(key) {
                ptr.mark_ptr(gray_stack);
            }
            value.mark(gray_stack);
        });
    }

    pub fn mark_open_upvalues(&mut self, gray_stack: &mut Vec<*mut Object>) {
        unsafe {
            let mut curr = self.open_upvalues;
            while !curr.is_null() {
                curr.mark_ptr(gray_stack);
                curr = (*curr).next;
            }
        }
    }

    pub fn remove_unmarked_strings(&mut self) {
        self.strings.retain(|_, v| unsafe { v.read().is_marked });
    }

    pub fn sweep(&mut self) {
        let mut previous = std::ptr::null_mut();
        let mut current = self.objects;
        unsafe {
            while !current.is_null() {
                if !(*current).is_marked {
                    (*current).is_marked = false;
                    previous = current;
                    current = (*current).next;
                } else {
                    let unreachable = current;
                    current = (*current).next;
                    if !previous.is_null() {
                        (*previous).next = (*unreachable).next;
                    } else {
                        self.objects = (*unreachable).next;
                    }
                    drop(Box::from_raw(unreachable));
                }
            }
        }
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

impl Drop for MemoryManager {
    fn drop(&mut self) {
        self.globals.clear();
        self.strings.clear();
        let mut curr = self.objects;
        unsafe {
            while !curr.is_null() {
                let next = (*curr).next;

                #[cfg(feature = "debug-log-gc")]
                println!("{:p} free type {:?}", curr, (*curr).data);

                drop(Box::from_raw(curr));
                curr = next;
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
            assert_eq!((*p3).next, p2);
            assert_eq!((*p2).next, p1);
            assert!((*p1).next.is_null());
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
    fn test_set_global_success() {
        let mut mm = MemoryManager::new();

        let key = mm.intern_string("a");
        let val1 = Value::Number(1.0);
        let val2 = Value::Number(2.0);

        mm.define_global(key, val1);
        let result = mm.set_global(key, val2);

        assert_eq!(result, true, "Should succeed when variable exists");
        assert_eq!(mm.get_global(key), Some(val2), "Value should have been updated");
    }

    #[test]
    fn test_set_global_failure_if_not_exists() {
        let mut mm = MemoryManager::new();
        let key = mm.intern_string("a");
        let val = Value::Number(1.0);

        let result = mm.set_global(key, val);
        assert_eq!(result, false, "Should fail if variable has not been defined");
    }
}
