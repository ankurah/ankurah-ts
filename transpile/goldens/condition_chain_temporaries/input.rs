//! Each condition of an `if`/`else if` chain is its own temporary scope. Rust
//! releases what a condition produced at the end of that condition, and a
//! condition is evaluated only once the one above it has failed — so the
//! statements that take and release a temporary stand inside the `else` that
//! leads to the condition needing them.

use std::sync::Mutex;

pub struct Reading {
    pub level: usize,
}

pub fn reading(level: usize) -> Reading {
    Reading { level }
}

pub struct Meter {
    pub floor: Mutex<usize>,
}

impl Meter {
    /// Two conditions build a `Reading` and the third takes a lock. Whichever
    /// branch is chosen, only the conditions above it ran, and each released
    /// what it took.
    pub fn band(&self, level: usize) -> usize {
        if reading(level).level > 10 {
            3
        } else if reading(level).level > 5 {
            2
        } else if *self.floor.lock().unwrap() > level {
            1
        } else {
            0
        }
    }

    /// A `while` condition is re-evaluated every turn, and every turn releases
    /// what that turn's condition built.
    pub fn climb(&self) -> usize {
        let mut level = 0usize;
        while reading(level).level < 3 {
            level += 1;
        }
        level
    }
}
