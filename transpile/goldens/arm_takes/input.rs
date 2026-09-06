//! What an arm's pattern takes out of the value it is written for, in the three
//! shapes the two sides of the port used to answer differently.
//!
//! A partially named tuple moves the elements it names and leaves the rest for
//! Rust to drop where the match ends. A struct-variant pattern names its members
//! and says nothing about where they are declared. And `Some` is the prelude's
//! `Some` or it is an ordinary variant of an ordinary enum, which the port
//! cannot take a name out of without leaving the wrapper unreleased.

pub struct Token { pub n: u32 }
impl Token { pub fn new(n: u32) -> Token { Token { n } } }
impl Drop for Token { fn drop(&mut self) {} }

/// One element named, one left: `pair.1` is dropped when the match ends.
pub fn partial(pair: (Token, Token)) -> u32 {
    match pair {
        (a, _) => { let n = a.n; drop(a); n }
    }
}

/// The control: nothing named, so the subject keeps its own release.
pub fn nothing(pair: (Token, Token)) -> u32 {
    match pair { (_, _) => 0 }
}

/// The control: every element named, so each binding releases its own.
pub fn both(pair: (Token, Token)) -> u32 {
    match pair { (a, b) => { let n = a.n + b.n; drop(a); drop(b); n } }
}

pub enum Named { V { copy: u32, held: Token }, Empty }

/// The arm names `held` first though `copy` is declared first: paired by
/// POSITION, `held` was read as the `u32`, the match was written as a borrow,
/// and the token was released by the binding and by the enum's cascade.
pub fn out_of_order(v: Named) -> u32 {
    match v {
        Named::V { held, .. } => { let n = held.n; drop(held); n }
        Named::Empty => 0,
    }
}

pub enum Maybe { Some(Token), None }
impl Drop for Maybe { fn drop(&mut self) {} }
pub enum Outer { W(Maybe), Nothing }

/// A user enum with a variant spelled `Some`. It is not the nullable, so taking
/// `t` out of it leaves a `Maybe` the port cannot release: refused, not leaked.
pub fn user_some(o: Outer) -> u32 {
    match o {
        Outer::W(Maybe::Some(t)) => t.n,
        _ => 0,
    }
}

pub enum Holder { Pair((Token, Token)), Nothing }

/// The partial tuple again, as a variant's member. Here the port does not carry
/// the member's element types, so it refuses rather than guessing.
pub fn member(h: Holder) -> u32 {
    match h {
        Holder::Pair((a, _)) => { let n = a.n; drop(a); n }
        Holder::Nothing => 0,
    }
}
