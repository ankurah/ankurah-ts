// A write to an atomic reached through the holder that carries it.
//
// An `AtomicUsize` is a number here and an `Arc` hands its contents out through
// an accessor that READS them, so `counter.fetch_add(1, ..)` would add to a copy
// and lose the answer. Reads through the same accessor are the value itself and
// stand. The write is a hole until the runtime has a cell an `Arc` can hold.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub fn read_back() -> usize {
    let counter = Arc::new(AtomicUsize::new(7));
    counter.load(Ordering::SeqCst)
}

pub fn bump() -> usize {
    let counter = Arc::new(AtomicUsize::new(0));
    let clone = counter.clone();
    clone.fetch_add(1, Ordering::SeqCst);
    counter.load(Ordering::SeqCst)
}
