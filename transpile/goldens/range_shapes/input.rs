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

/// O7/O8: `contains` is ONE helper, with the arguments in Rust's order, and it
/// answers every bound shape. Written inline as `start <= item && item < end`
/// the item was evaluated twice and the end only where the first comparison
/// held; and `a..`, `..b`, `..=b` and `..` each fell to the materialisation
/// hole though their answers are `a <= x`, `x < b`, `x <= b` and `true`.
pub fn side() -> u32 {
    3
}

pub fn once_only(n: u32) -> bool {
    (0u32..n).contains(&side())
}

pub fn from_five(x: u32) -> bool {
    (5u32..).contains(&x)
}

pub fn up_to_five(x: u32) -> bool {
    (..5u32).contains(&x)
}

pub fn up_to_five_incl(x: u32) -> bool {
    (..=5u32).contains(&x)
}

pub fn anything(x: u32) -> bool {
    (..).contains(&x)
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

/// N5: and the same answer through a BORROWED chain, where the element comes
/// back as `&Option<u32>`. Asked about the reference rather than about what it
/// points at, the port read `&Option` as "not a nullable" and flattened the two
/// `null`s with no diagnostic at all — while `first` above, and the OWNED
/// spelling of this very reader, refused correctly.
pub fn found_slot(slots: &Vec<Option<u32>>) -> Option<&Option<u32>> {
    slots.iter().find(|s| s.is_some())
}

/// The consuming spelling, which has always refused.
pub fn taken_slot(slots: Vec<Option<u32>>) -> Option<Option<u32>> {
    slots.into_iter().find(|s| s.is_some())
}

/// And the same chain over a plain element still answers.
pub fn found_plain(ns: &Vec<u32>) -> Option<&u32> {
    ns.iter().find(|n| **n > 7)
}
