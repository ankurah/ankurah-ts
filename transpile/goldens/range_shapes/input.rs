//! What a range IS in the port, and what it is not. F3, E7, E13.
//!
//! The port has no `Range` type, so a bounded range is MATERIALISED — the
//! sequence of its values — which is what makes `rev`, `map`, `filter` and
//! `contains` array operations here. The check that decided which ranges could
//! be built named the one width it could NOT count (a `bigint`) and let
//! everything else through, so:
//!
//!   - `('a'..='c')` came out as `rangeIncl('a', 'c')`, which is `["a"]`,
//!     because `'a' + 1` is the string `"a1"` and `"a1" <= "c"` is false;
//!   - `(0.0f64..1.0f64).contains(&0.5)` came out as `range(0, 1).contains(0.5)`
//!     — `range(0, 1)` is `[0]`, and an array has no `contains`, so the site was
//!     a `TypeError` and no diagnostic named it;
//!   - `step_by` had no lowering at all and came out as `(range(0, 10)).stepBy(2)`,
//!     a method no array declares.
//!
//! So only the discrete integer widths `n++` steps are built, `contains` is
//! written from the BOUNDS — a float range is not an iterator in Rust either,
//! and `contains` is the one thing it still answers — and `step_by` is every
//! nth element of the sequence.

/// A float range answers `contains` and nothing else, and it is answered
/// against the two ends rather than against a sequence.
pub fn within_unit(x: f64) -> bool {
    (0.0f64..1.0f64).contains(&x)
}

/// The same for an integer range, half-open and closed.
pub fn within_16(x: u32) -> bool {
    (0u32..16u32).contains(&x)
}

pub fn up_to_16(x: u32) -> bool {
    (0u32..=16u32).contains(&x)
}

/// `step_by` keeps every nth value of the sequence.
pub fn evens_to_ten() -> Vec<u32> {
    (0u32..10u32).step_by(2).collect()
}

/// A `char` range is the sequence of its code points, which the port has no
/// helper for: refused rather than built out of string comparisons.
pub fn letters() -> Vec<char> {
    ('a'..='c').collect::<Vec<char>>()
}

/// E13: `Option<T>` is `T | null` here, so a reader answering `Option<Element>`
/// over a vector of `Option`s has ONE `null` for two different answers.
pub fn first_slot(slots: &Vec<Option<u32>>) -> Option<Option<u32>> {
    slots.first().copied()
}

/// The same reader over a plain element is unchanged.
pub fn first_plain(ns: &Vec<u32>) -> Option<u32> {
    ns.first().copied()
}
