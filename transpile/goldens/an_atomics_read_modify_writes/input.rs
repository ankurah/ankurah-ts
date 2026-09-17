// Every read-modify-write the declared atomic surface offers.
//
// Each answers the value it FOUND and stores the operator's own answer, so a
// method the surface declares and the lowering does not write reaches
// JavaScript as a method no number has. `AtomicBool`'s three are the logical
// operators; the integer ones are the bitwise operators and two comparisons.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct Counter {
    pub bits: AtomicUsize,
}

impl Counter {
    pub fn new() -> Counter {
        Counter { bits: AtomicUsize::new(0b1010) }
    }

    pub fn set_low_bit(&self) -> usize {
        self.bits.fetch_or(0b0001, Ordering::SeqCst)
    }
}

pub fn bitwise_over_a_local() -> Vec<usize> {
    let flags = AtomicUsize::new(0b1100);
    let was_or = flags.fetch_or(0b0011, Ordering::SeqCst);
    let was_and = flags.fetch_and(0b0110, Ordering::SeqCst);
    let was_xor = flags.fetch_xor(0b0101, Ordering::SeqCst);
    vec![was_or, was_and, was_xor, flags.load(Ordering::SeqCst)]
}

pub fn bounds_over_a_local() -> Vec<usize> {
    let seen = AtomicUsize::new(5);
    let was_max = seen.fetch_max(9, Ordering::SeqCst);
    let after_max = seen.load(Ordering::SeqCst);
    let was_min = seen.fetch_min(2, Ordering::SeqCst);
    vec![was_max, after_max, was_min, seen.load(Ordering::SeqCst)]
}

pub fn logic_over_a_local() -> Vec<bool> {
    let ready = AtomicBool::new(true);
    let was_and = ready.fetch_and(false, Ordering::SeqCst);
    let was_or = ready.fetch_or(true, Ordering::SeqCst);
    let was_xor = ready.fetch_xor(true, Ordering::SeqCst);
    vec![was_and, was_or, was_xor, ready.load(Ordering::SeqCst)]
}

pub fn through_a_field() -> Vec<usize> {
    let counter = Counter::new();
    let was = counter.set_low_bit();
    vec![was, counter.bits.load(Ordering::SeqCst)]
}
