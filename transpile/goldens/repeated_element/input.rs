//! `vec![v; n]`: what the port can write, and what it must refuse.
//!
//! Rust CLONES the value into every slot. `Array(n).fill(v)` puts the same one
//! in each, which is the same program only where the value has no identity to
//! share — a number, a string, a boolean, a `bigint`. Anything the port writes
//! as a JavaScript reference is a different program: one object with n names
//! against n objects, so the shape is a hole rather than emitted code.
//!
//! And the repeated value stands at the sequence's ELEMENT type, exactly as
//! each value of the comma-list form does. Written without that expectation,
//! `vec![true.into(); 2]` under a `Vec<Held>` lost the conversion entirely and
//! put two `true`s in the array.

pub struct Held {
    pub flag: bool,
}

impl From<bool> for Held {
    fn from(flag: bool) -> Held {
        Held { flag }
    }
}

impl Clone for Held {
    fn clone(&self) -> Held {
        Held { flag: self.flag }
    }
}

/// A value with an identity, repeated: refused.
pub fn shared() -> Vec<Held> {
    vec![true.into(); 2]
}

/// The comma-list form, where each value is its own.
pub fn listed() -> Vec<Held> {
    vec![true.into(), false.into()]
}

/// A number has nothing to share, and `fill` IS the clone.
pub fn numbers() -> Vec<u32> {
    vec![7u32; 3]
}

/// So has a string.
pub fn words() -> Vec<String> {
    vec![String::new(); 2]
}
