//! What an eager adaptor discards, and what a key fold releases. O3, O4, O13.
//!
//! For: Rust's adaptors are lazy and own what they walk, so `Filter` drops the
//! element its predicate rejected, `Skip` drops the prefix it walked past,
//! `Take` drops the tail with the iterator it wraps, and `StepBy` drops what it
//! stepped over. The port writes them eagerly, as array operations, and those
//! simply lost the discarded elements — after which the consuming terminal
//! below could not release what the adaptor had already erased. A borrowed
//! chain has none of this: `iter()` hands out references and the sequence is
//! somebody else's.
//!
//! `max_by_key` is Rust's `self.map(|x| (f(&x), x)).max_by(..)` in both
//! ownership modes, and the KEYS are built by the fold whichever mode it is in:
//! the loser's pair is dropped, so its key goes with it, and the winner's pair
//! is destructured, so its key is dropped where the element comes out. The
//! reading fold released none of them.

pub struct Token(pub u32);

impl Drop for Token {
    fn drop(&mut self) {}
}

/// A key with drop glue of its own, so that "the fold released the key" is a
/// checkable property and not a claim about a number.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Key(pub u32);

impl Drop for Key {
    fn drop(&mut self) {}
}

/// O4: `Filter` drops what its predicate rejects, and the terminal below it
/// never sees those elements at all.
pub fn first_over(tokens: Vec<Token>, want: u32) -> Option<Token> {
    tokens.into_iter().filter(|t| t.0 > 0).find(|t| t.0 == want)
}

/// O3: `StepBy` drops what it stepped over.
pub fn every_other(tokens: Vec<Token>) -> Option<Token> {
    tokens.into_iter().step_by(2).last()
}

/// `Skip` drops the prefix and `Take` drops the tail.
pub fn middle(tokens: Vec<Token>) -> Option<Token> {
    tokens.into_iter().skip(1).take(1).last()
}

/// A borrowed chain discards nothing: the sequence is still the caller's.
pub fn borrowed_filter(tokens: &Vec<Token>, want: u32) -> Option<&Token> {
    tokens.iter().filter(|t| t.0 > 0).find(|t| t.0 == want)
}

/// O13: the reading key fold builds a key per element and released none of
/// them — not the displaced ones and not the winner's.
pub fn widest(tokens: &Vec<Token>) -> Option<&Token> {
    tokens.iter().max_by_key(|t| Key(t.0))
}

/// And the consuming one, which releases the losers as well as their keys.
pub fn widest_owned(tokens: Vec<Token>) -> Option<Token> {
    tokens.into_iter().max_by_key(|t| Key(t.0))
}

/// Q1: `next` on a receiver nobody else holds hands back the head, and the
/// iterator — dropped at the end of the statement — drops the rest.
pub fn first_owned(tokens: Vec<Token>) -> Option<Token> {
    tokens.into_iter().next()
}

/// The same on a borrowed chain reads through and releases nothing.
pub fn first_borrowed(tokens: &Vec<Token>) -> Option<&Token> {
    tokens.iter().next()
}
