//! A guard produced inside an expression and bound to nothing: Rust drops it at
//! the end of the statement, so the emitter gives it a name, releases it there,
//! and lists it in the enclosing `finally` as well. A guard's second drop is a
//! deliberate no-op, which is what makes the pair safe.

use std::sync::Mutex;

pub struct Counter {
    pub value: Mutex<usize>,
}

impl Counter {
    /// The guard is the receiver of a borrowing call and nothing binds it.
    pub fn read(&self) -> usize {
        let seen = *self.value.lock().unwrap();
        seen + 1
    }

    /// A guard the source does bind is released the same way, without a
    /// temporary of its own.
    pub fn bump(&self) -> usize {
        let mut guard = self.value.lock().unwrap();
        *guard += 1;
        *guard
    }
}
