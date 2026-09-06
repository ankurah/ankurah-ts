//! Two operators the port used to write as JavaScript's own, and one pattern it
//! had no test for. K8, K9.
//!
//! For: a signed width's `MIN` has no positive of its own — `-i32::MIN` does not
//! fit in an `i32` — and Rust's debug build panics there. JavaScript's `-`
//! answers `2147483648`, which no `i32` holds, and says nothing; `abs()` has
//! gone through the runtime's `checkedNeg` since Z8, and `-` did not. And
//! `Variant(..)` matches every value of that variant, which the pattern
//! translator had no test to write for: it wrote a HOLE, so the arm threw
//! before the body the source wrote could run.

pub fn negate(n: i32) -> i32 {
    -n
}

pub fn negate_wide(n: i64) -> i64 {
    -n
}

/// A float keeps the operator: IEEE negation is total, and `f64::MIN` is not
/// its own edge case.
pub fn negate_float(x: f64) -> f64 {
    -x
}

/// A LITERAL keeps it too. `-2147483648` is how `i32::MIN` is written, and
/// negating the literal `2147483648` through the helper would raise on exactly
/// the value the source is naming.
pub fn smallest() -> i32 {
    -2147483648
}

pub enum Wide {
    Two(u32, u32),
    One(u32),
    Nothing,
}

/// `Variant(..)`: no name taken, and no test to make — the variant key IS the
/// test.
pub fn covered(w: &Wide) -> u32 {
    match w {
        Wide::Two(..) => 2,
        Wide::One(n) => *n,
        Wide::Nothing => 0,
    }
}

/// A trailing `..` after a name: the names before it take the members at their
/// own positions, and the `..` covers the rest.
pub fn first_of(w: &Wide) -> u32 {
    match w {
        Wide::Two(a, ..) => *a,
        Wide::One(n) => *n,
        Wide::Nothing => 0,
    }
}
