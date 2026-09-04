//! A guard taken inside a condition is that condition's temporary even when the
//! condition only reads a field off it. Rust releases it at the end of the
//! condition, before the branch is taken, which is what lets the body lock the
//! same mutex again.

use std::sync::Mutex;

pub struct Slot {
    pub n: usize,
}

pub struct Cell {
    pub slot: Mutex<Slot>,
}

impl Cell {
    /// The guard is not the condition's own value — the condition reads `n` off
    /// it — and the body takes the same lock, which it can only do because the
    /// condition's guard is already gone.
    pub fn clear(&self) -> bool {
        if self.slot.lock().unwrap().n > 0 {
            let mut guard = self.slot.lock().unwrap();
            guard.n = 0;
            return true;
        }
        false
    }

    /// The condition runs afresh every turn, and every turn's guard is released
    /// before the body takes its own.
    pub fn drain(&self) -> usize {
        let mut turns = 0usize;
        while self.slot.lock().unwrap().n > 0 {
            let mut guard = self.slot.lock().unwrap();
            guard.n -= 1;
            turns += 1;
        }
        turns
    }
}
