//! An `if let` whose pattern takes a payload out, in the four shapes the
//! routing used to miss.
//!
//! An OR-pattern is one pattern and two variants, and the routing read the
//! whole pattern's leaf: `Shape::One(t) | Shape::Two(t, _)` is neither a tuple
//! struct nor a path, so the match went to the if-chain, which reads `.value`
//! in place and marks nothing moved. Nobody released the enum, and nobody
//! released the `Two` payload the pattern did not name.
//!
//! A nested ordinary struct was asked which VARIANT it was — `v._0.is('Token')`
//! is a method no struct has — so the branch threw with the value it had opened
//! still live.
//!
//! A crate's own `Ok` variant was opened as the runtime's `Result`, so
//! `o.unwrap()` threw on a class that has no `unwrap` and the enum stayed live.
//!
//! And a genuine `Result` reached through an iterator's `Item` is still the
//! wrapper, which is the other half of the same question.

pub struct Token {
    pub n: i64,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

pub enum Shape {
    One(Token),
    Two(Token, i64),
    Nothing,
}

pub enum Duo {
    A(Token),
    B,
}

pub enum Outcome {
    Ok(Token),
    Other,
}

/// Both alternatives take the payload out, and the second leaves a member the
/// pattern did not name.
pub fn either(s: Shape) -> i64 {
    if let Shape::One(t) | Shape::Two(t, _) = s {
        t.n
    } else {
        0
    }
}

/// The nested pattern names a struct, so its fields are read off the object.
pub fn nested_struct(d: Duo) -> i64 {
    if let Duo::A(Token { n }) = d {
        n
    } else {
        0
    }
}

/// `Outcome::Ok` is the crate's own variant.
pub fn user_ok(o: Outcome) -> i64 {
    if let Outcome::Ok(t) = o {
        t.n
    } else {
        0
    }
}

/// And the wrapper is still the wrapper, reached through a bound.
pub fn first_ok<I>(mut items: I) -> i64
where
    I: Iterator<Item = Result<Token, Token>>,
{
    if let Some(r) = items.next() {
        if let Ok(t) = r {
            t.n
        } else {
            0
        }
    } else {
        0
    }
}
