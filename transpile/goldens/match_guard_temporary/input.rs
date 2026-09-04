//! A `match` arm's guard reads the names its own pattern bound and takes
//! temporaries of its own. So the bindings stand above the guard, the guard's
//! temporaries are released as soon as the guard has been tested, and an arm
//! whose guard failed hands the subject to the arm below it.

pub struct Reading {
    pub limit: usize,
}

pub fn limit_of(scale: usize) -> Reading {
    Reading { limit: scale * 2 }
}

/// `n` is the name the pattern bound and the guard reads; the `Reading` the
/// guard builds is what Rust drops before the arm's body runs.
pub fn classify(value: usize, scale: usize) -> usize {
    match value {
        0 => 0,
        n if limit_of(scale).limit > n => 1,
        _ => 2,
    }
}

/// Two guarded arms in a row: the first guard's temporary is released whether
/// its test passed or failed, and a guard that failed leaves the subject to the
/// arm below it.
pub fn banded(value: usize, scale: usize) -> usize {
    match value {
        n if limit_of(scale).limit > n => 1,
        n if limit_of(scale * 4).limit > n => 2,
        _ => 3,
    }
}
