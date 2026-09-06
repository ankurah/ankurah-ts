//! The iterator terminals that TAKE what they walk. F1, F9.
//!
//! For: Rust's consuming terminals own the sequence's elements — the one they
//! select becomes the caller's and every other one is dropped where Rust drops
//! it — and the port wrote every one of them as a reading helper over an array
//! somebody else released. Which of two wrong answers came out depended on
//! whether Rust's signature happens to say `self` or `&mut self` about the
//! ITERATOR, which says nothing at all about the items:
//!
//!   - `&mut self` (`find`, `position`, `find_map`) was hoisted and released,
//!     so `find` handed back an element of a sequence the `finally` then
//!     released, and a `position` closure that dropped its own element hit
//!     `OwnershipFatal` on the second drop;
//!   - `self` (`max_by_key`, `min_by_key`, `reduce`, `last`) was not hoisted at
//!     all, so every element the terminal did not hand back leaked.
//!
//! A BORROWED chain is the other half and must not change: `iter()` hands out
//! `&T`, and the sequence belongs to whoever it was read from.

pub struct Token(pub u32);

impl Drop for Token {
    fn drop(&mut self) {}
}

/// `&mut self`, and a closure that takes the element BY VALUE. The parent
/// engine dropped it a second time in the `finally`.
pub fn position_of(tokens: Vec<Token>, want: u32) -> Option<usize> {
    tokens.into_iter().position(|token| {
        let hit = token.0 == want;
        drop(token);
        hit
    })
}

/// `&mut self`, and a predicate that only BORROWS. The element the call answers
/// is the caller's; every other one is the terminal's to drop.
pub fn find_one(tokens: Vec<Token>, want: u32) -> Option<Token> {
    tokens.into_iter().find(|t| t.0 == want)
}

/// `self`: the losers leaked.
pub fn biggest(tokens: Vec<Token>) -> Option<Token> {
    tokens.into_iter().max_by_key(|t| t.0)
}

/// The comparator family, whose comparator only borrows.
pub fn smallest(tokens: Vec<Token>) -> Option<Token> {
    tokens.into_iter().min_by(|a, b| a.0.cmp(&b.0))
}

/// `reduce` hands BOTH values to the closure, which is then the only thing that
/// can drop either.
pub fn first_kept(tokens: Vec<Token>) -> Option<Token> {
    tokens.into_iter().reduce(|a, b| {
        drop(b);
        a
    })
}

/// `Iterator::last(self)` walks the whole sequence and drops all but the end.
pub fn last_of(tokens: Vec<Token>) -> Option<Token> {
    tokens.into_iter().last()
}

/// `slice::last(&self)` is a different method under the same name: it borrows,
/// and the sequence stays the caller's.
pub fn peek_last(tokens: &Vec<Token>) -> Option<&Token> {
    tokens.last()
}

/// A borrowed chain: nothing here owns anything.
pub fn borrowed_find(tokens: &Vec<Token>, want: u32) -> Option<&Token> {
    tokens.iter().find(|t| t.0 == want)
}

/// Elements with no drop glue: the reading helper is right for them too.
pub fn first_even(ns: Vec<u32>) -> Option<u32> {
    ns.into_iter().find(|n| *n % 2 == 0)
}
