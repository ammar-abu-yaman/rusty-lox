use std::fmt::Display;

use super::object::Object;
/// Quite NAN constant as a 64 bit
const QNAN: u64 = 0x7ffc000000000000u64;

const SIGN_BIT: u64 = 0x8000000000000000u64;
const TAG_NIL: u64 = 1;
const TAG_FALSE: u64 = 2;
const TAG_TRUE: u64 = 3;

const NIL_VALUE: u64 = QNAN | TAG_NIL;
const TRUE_VALUE: u64 = QNAN | TAG_TRUE;
const FALSE_VALUE: u64 = QNAN | TAG_FALSE;

#[cfg(not(feature = "nan-boxing"))]
#[derive(Debug, Clone, Copy)]
pub enum Value {
    Number(f64),
    Bool(bool),
    Object(*mut Object),
    Nil,
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_number() {
            write!(f, "{}", self.as_number())
        } else if self.is_bool() {
            write!(f, "{}", self.as_bool())
        } else if self.is_nil() {
            write!(f, "nil")
        } else {
            write!(f, "{}", self.as_object())
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        #[cfg(not(feature = "nan-boxing"))]
        return match (self, other) {
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Object(a), Value::Object(b)) => a == b || unsafe { (**a).data == (**b).data },
            _ => false,
        };
        #[cfg(feature = "nan-boxing")]
        {
            if self.is_number() && other.is_number() {
                return self.as_number() == other.as_number();
            }
            if self.is_object() && other.is_object() {
                return self.0 == other.0 || self.as_object().data == other.as_object().data;
            }
            return self.0 == other.0;
        }
    }
}

impl Value {
    pub fn as_number(&self) -> f64 {
        #[cfg(not(feature = "nan-boxing"))]
        return match self {
            Self::Number(n) => *n,
            _ => unreachable!("expected number, got {self}"),
        };
        #[cfg(feature = "nan-boxing")]
        return f64::from_bits(self.0);
    }

    pub fn as_object(&self) -> &Object {
        #[cfg(not(feature = "nan-boxing"))]
        return match self {
            Self::Object(o) => unsafe { o.as_ref_unchecked() },
            _ => unreachable!("expected object, got {self}"),
        };
        #[cfg(feature = "nan-boxing")]
        unsafe {
            let address = (self.0) & !(SIGN_BIT | QNAN);
            return &*(address as *const Object);
        }
    }

    pub fn as_object_ptr(&self) -> *mut Object {
        #[cfg(not(feature = "nan-boxing"))]
        match self {
            Self::Object(o) => *o,
            _ => unreachable!("expected object, got {self}"),
        }
        #[cfg(feature = "nan-boxing")]
        {
            let address = (self.0) & !(SIGN_BIT | QNAN);
            return address as *mut Object;
        }
    }

    pub fn as_mut_object(&mut self) -> &mut Object {
        #[cfg(not(feature = "nan-boxing"))]
        match self {
            Self::Object(o) => unsafe { o.as_mut_unchecked() },
            _ => unreachable!("expected object, got {self}"),
        }
        #[cfg(feature = "nan-boxing")]
        unsafe {
            let address = (self.0) & !(SIGN_BIT | QNAN);
            return &mut *(address as *mut Object);
        }
    }

    pub fn as_bool(&self) -> bool {
        #[cfg(not(feature = "nan-boxing"))]
        return match self {
            Value::Bool(b) => *b,
            _ => unreachable!("expected object, got {self}"),
        };
        #[cfg(feature = "nan-boxing")]
        return self.0 == TRUE_VALUE;
    }

    pub fn is_bool(&self) -> bool {
        #[cfg(not(feature = "nan-boxing"))]
        return match self {
            Value::Bool(_) => true,
            _ => false,
        };
        #[cfg(feature = "nan-boxing")]
        return self.0 == TRUE_VALUE || self.0 == FALSE_VALUE;
    }

    pub fn is_nil(&self) -> bool {
        #[cfg(not(feature = "nan-boxing"))]
        return match self {
            Value::Nil => true,
            _ => false,
        };
        #[cfg(feature = "nan-boxing")]
        return self.0 == NIL_VALUE;
    }

    pub fn is_falsy(&self) -> bool {
        #[cfg(not(feature = "nan-boxing"))]
        return matches!(self, Self::Nil | Self::Bool(false));
        #[cfg(feature = "nan-boxing")]
        return self.0 == NIL_VALUE || self.0 == FALSE_VALUE;
    }

    pub fn is_number(&self) -> bool {
        #[cfg(not(feature = "nan-boxing"))]
        return matches!(self, Self::Number(_));
        #[cfg(feature = "nan-boxing")]
        return (self.0 & QNAN) != QNAN;
    }

    pub fn is_object(&self) -> bool {
        #[cfg(not(feature = "nan-boxing"))]
        return matches!(self, Self::Object(_));
        #[cfg(feature = "nan-boxing")]
        return ((self.0) & (QNAN | SIGN_BIT)) == (QNAN | SIGN_BIT);
    }

    pub fn is_string_object(&self) -> bool {
        self.is_object() && self.as_object().is_string()
    }

    pub fn mark(&mut self, gray_stack: &mut Vec<*mut Object>) {
        #[cfg(not(feature = "nan-boxing"))]
        if let Self::Object(ptr) = self {
            ptr.mark_ptr(gray_stack);
        }
        #[cfg(feature = "nan-boxing")]
        if self.is_object() {
            self.as_object_ptr().mark_ptr(gray_stack);
        }
    }
}

#[cfg(feature = "nan-boxing")]
#[derive(Debug, Clone, Copy)]
pub struct Value(u64);

impl Value {
    pub fn nil_value() -> Self {
        #[cfg(not(feature = "nan-boxing"))]
        return Value::Nil;
        #[cfg(feature = "nan-boxing")]
        return Value(NIL_VALUE);
    }

    pub fn true_value() -> Self {
        #[cfg(not(feature = "nan-boxing"))]
        return Value::Bool(true);
        #[cfg(feature = "nan-boxing")]
        return Value(TRUE_VALUE);
    }

    pub fn false_value() -> Self {
        #[cfg(not(feature = "nan-boxing"))]
        return Value::Bool(false);
        #[cfg(feature = "nan-boxing")]
        return Value(FALSE_VALUE);
    }

    pub fn bool(value: bool) -> Self {
        if value {
            Self::true_value()
        } else {
            Self::false_value()
        }
    }

    pub fn number(value: f64) -> Self {
        #[cfg(not(feature = "nan-boxing"))]
        return Value::Number(value);
        #[cfg(feature = "nan-boxing")]
        return Value(value.to_bits());
    }

    pub fn object(ptr: *mut Object) -> Self {
        #[cfg(not(feature = "nan-boxing"))]
        return Value::Object(ptr);
        #[cfg(feature = "nan-boxing")]
        return Value(SIGN_BIT | QNAN | (ptr as u64));
    }
}
