//! A `range.contains(&x)` over a type whose order is PARTIAL, with bounds the
//! expression builds — and an owning adaptor on a named iterator beside it.
//!
//! The range is never materialised: the comparison is written from its BOUNDS,
//! which is what lets a range of a width the port cannot count answer at all.
//! So the receiver position writes nothing — and hoisting nothing wrote
//! `const _t0 = ;`, a line no JavaScript engine will parse. What the bounds owe
//! is written where the bounds are, in Rust's order, and released around the
//! statement.
//!
//! The comparison itself is `PartialOrd`'s. A user `PartialOrd` that does not
//! forward to `Ord` is emitted as `partialCompareTo(): number | null`, and the
//! range helper knew only `compareTo`, so every such range threw "declares no
//! order". `null` there is Rust's "these two are not ordered", which fails
//! whichever of the four comparisons it was asked.
//!
//! And an ADAPTOR takes the iterator by value whatever it was called on, so a
//! bare named iterator is moved into it and the frame stops owning it. Refused
//! as though the caller kept part of it, `it.filter(p)` was a hole with a
//! release of `it` beside it.

pub struct Version(pub Option<u32>);

impl PartialEq for Version {
    fn eq(&self, other: &Version) -> bool {
        match (self.0, other.0) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Version) -> Option<std::cmp::Ordering> {
        match (self.0, other.0) {
            (Some(a), Some(b)) => a.partial_cmp(&b),
            _ => None,
        }
    }
}

impl Drop for Version {
    fn drop(&mut self) {}
}

pub fn version(n: u32) -> Version {
    Version(Some(n))
}

pub fn unknown() -> Version {
    Version(None)
}

/// Both bounds are built here, and Rust drops them with the range.
pub fn between(v: &Version) -> bool {
    (version(1)..version(9)).contains(v)
}

/// A bound that names a place builds nothing.
pub fn between_names(lo: &Version, hi: &Version, v: &Version) -> bool {
    (lo..hi).contains(&v)
}

pub struct Token {
    pub n: u32,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

/// The adaptor moves the named iterator into itself.
pub fn kept(xs: Vec<Token>) -> Vec<Token> {
    let it = xs.into_iter();
    it.filter(|t| t.n > 1).collect()
}
