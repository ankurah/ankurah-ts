//! A guard taken in a condition is a temporary of that condition. Rust releases
//! it before the block runs — an `if` condition's temporaries die at the end of
//! the condition, and a `while` condition's die at the end of every turn's
//! condition — which is what lets each body lock the same mutex again.

use std::sync::Mutex;

pub struct Counter {
    pub value: Mutex<usize>,
}

impl Counter {
    /// The body locks the same mutex the condition locked. It can only do that
    /// because the condition's guard is already gone.
    pub fn start_if_idle(&self) -> bool {
        if *self.value.lock().unwrap() == 0 {
            let mut guard = self.value.lock().unwrap();
            *guard = 1;
            return true;
        }
        false
    }

    /// The condition runs afresh every turn, and every turn's guard is released
    /// before the body takes its own.
    pub fn wind_down(&self) -> usize {
        let mut turns = 0usize;
        while *self.value.lock().unwrap() > 0 {
            let mut guard = self.value.lock().unwrap();
            *guard -= 1;
            turns += 1;
        }
        turns
    }
}
