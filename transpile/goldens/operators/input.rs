//! Operators, once the operands are named.
//!
//! JavaScript's operators are not Rust's: `==` on two objects compares
//! identity, integer division leaves a fraction behind, `!` on a number is a
//! boolean, and a `bigint` beside a `number` throws rather than adding. Each of
//! those is a wrong answer the emitted code used to give silently.

#[derive(Clone, PartialEq)]
pub struct Tag {
    pub id: u32,
}

/// `==` between two values of a type that derives `PartialEq` is the `equals`
/// the derive emitted, not a reference comparison.
pub fn same(a: &Tag, b: &Tag) -> bool {
    a == b
}

pub fn different(a: &Tag, b: &Tag) -> bool {
    a != b
}

/// Integer division truncates towards zero in Rust and does not in JavaScript.
pub fn halves(n: u32) -> u32 {
    n / 2
}

/// `!` on an integer flips its bits; JavaScript spells that `~`.
pub fn flipped(bits: u32) -> u32 {
    !bits
}

/// `!` on a boolean is the negation both languages spell the same way.
pub fn negated(yes: bool) -> bool {
    !yes
}

/// 64-bit arithmetic is `bigint` arithmetic, and a literal written against one
/// has to be a `bigint` too.
pub fn shifted(bits: u64) -> u64 {
    bits ^ (1 << 63)
}

/// Comparison and arithmetic on ordinary numbers stay what they were.
pub fn bigger(a: u32, b: u32) -> bool {
    a > b
}
