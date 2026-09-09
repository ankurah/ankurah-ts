// An atomic IS the value it holds in this port, so every write through the
// shared reference Rust allows is a write to the binding or the field.
//
// Declared `const` and `readonly`, those writes threw at run time and would not
// compile. `swap` and `compare_exchange` answer what the place HELD: read as a
// bare boolean, the old value was lost and every use of the Result had nothing.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct Counter {
    pub hits: AtomicUsize,
    pub alive: AtomicBool,
}

impl Counter {
    pub fn new() -> Counter {
        Counter { hits: AtomicUsize::new(0), alive: AtomicBool::new(true) }
    }

    pub fn bump(&self) -> usize {
        self.hits.fetch_add(1, Ordering::SeqCst)
    }

    pub fn read(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }

    pub fn close(&self) -> bool {
        self.alive.swap(false, Ordering::SeqCst)
    }

    pub fn claim(&self) -> bool {
        self.alive
            .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
}

pub fn counted() -> usize {
    let counter = Counter::new();
    counter.bump();
    counter.bump();
    counter.read()
}

pub fn closed() -> bool {
    let counter = Counter::new();
    counter.close()
}

pub fn claimed_twice() -> bool {
    let counter = Counter::new();
    let first = counter.claim();
    let second = counter.claim();
    first && !second
}

pub fn stored() -> usize {
    let held = AtomicUsize::new(0);
    held.store(3, Ordering::SeqCst);
    held.load(Ordering::SeqCst)
}
