use std::fmt::Display;

use crate::class::Class;

#[derive(Debug, Clone)]
pub struct Instance {
    class: Class,
}

impl Instance {
    pub fn new(class: Class) -> Self {
        Self { class }
    }
}

impl Display for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} instance", self.class)
    }
}

impl PartialEq for Instance {
    fn eq(&self, other: &Self) -> bool {
        self.class == other.class
    }
}

impl PartialOrd for Instance {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.class.partial_cmp(&other.class)
    }
}