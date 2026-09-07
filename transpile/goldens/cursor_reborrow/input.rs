//! Leg A: a cursor REBORROWED is not a cursor consumed.
//!
//! `Iterator::by_ref(&mut self) -> &mut Self` is a borrowed view of the walk,
//! and `&mut I` is itself an `Iterator` by the blanket impl — so a body handed
//! one may run the whole of `Iterator` on it, and Rust leaves the iterator
//! alive for its owner afterwards. The port has no reference, so the view IS
//! the cursor: written as a consumption, `walk.by_ref()` took the whole rest
//! out and marked the cursor moved, the `for` loop over it asked the same
//! question a second time (`walk.takeRest().takeRest()`, a method an array has
//! not got), and the owner's own `walk.drop()` was then a use after move.
//!
//! So a reborrow empties the cursor without taking it, and the owner's drop
//! finds it holding nothing.

pub struct Token {
    pub n: i64,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

/// The loop drains the walk; the cursor is still this body's, and this body
/// drops it.
pub fn sum_of<I>(mut walk: I) -> i64
where
    I: Iterator<Item = Token>,
{
    let mut total = 0i64;
    for token in walk.by_ref() {
        total += token.n;
    }
    total
}

/// A callee handed the reborrow drains it and hands the elements back.
pub fn drain<I>(values: &mut I) -> Vec<Token>
where
    I: Iterator<Item = Token>,
{
    values.collect()
}

/// And the OWNER still holds the cursor and drops it.
pub fn drained<I>(mut walk: I) -> i64
where
    I: Iterator<Item = Token>,
{
    let rest = drain(&mut walk);
    let mut total = 0i64;
    for token in rest {
        total += token.n;
    }
    total
}

pub fn summed(tokens: Vec<Token>) -> i64 {
    sum_of(tokens.into_iter())
}

pub fn all_drained(tokens: Vec<Token>) -> i64 {
    drained(tokens.into_iter())
}
