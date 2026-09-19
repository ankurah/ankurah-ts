//! `cloned` over a payload the engine cannot name still hands back a copy.
//!
//! The copy used to be gated on the payload owing a release, and a type
//! parameter owes nothing the engine can see. The caller's payload is
//! concrete, so the caller writes the release — and released the collection's
//! own value.

use std::collections::HashMap;

pub struct Holder {
    pub n: u64,
}

impl Drop for Holder {
    fn drop(&mut self) {}
}

impl Clone for Holder {
    fn clone(&self) -> Holder {
        Holder { n: self.n }
    }
}

pub struct Bag<V: Clone> {
    pub inner: HashMap<u64, V>,
}

impl<V: Clone> Bag<V> {
    pub fn get(&self, k: &u64) -> Option<V> {
        self.inner.get(k).cloned()
    }

    pub fn all(&self) -> Vec<V> {
        self.inner.values().cloned().collect()
    }
}

/// The monomorphic caller, whose local has a concrete type and is released.
pub fn take_one(bag: &Bag<Holder>, k: u64) -> u64 {
    match bag.get(&k) {
        Some(held) => held.n,
        None => 0,
    }
}

/// The sequence form of the same question.
pub fn take_all(bag: &Bag<Holder>) -> u64 {
    let mut total = 0u64;
    for held in bag.all() {
        total += held.n;
    }
    total
}
