//! An `Option` combinator's CLOSURE argument, and the order things run in.
//!
//! A `move` closure over something with drop glue is an `OwnedClosure`, which
//! is deliberately not callable as a function — and the combinators
//! interpolated it as `(new OwnedClosure(..))(value)`, a `TypeError` on the
//! first value each of them saw. Rust also builds the closure BEFORE it enters
//! the method and drops it on the path that does not call it, and the port
//! built it only on the selected path, so its captures leaked on the other one.
//! R10: an `OwnedClosure` is reached through `invoke`, and the branch that does
//! not call it releases it.
//!
//! And the receiver is evaluated before the arguments. Its name used to be
//! taken while the CALL was written — after them — so an argument that hoisted
//! anything landed ahead of it.

pub struct Token { pub n: u32 }
impl Token { pub fn new(n: u32) -> Token { Token { n } } }

pub fn source(n: u32) -> Option<u32> { if n == 0 { None } else { Some(n) } }
pub fn eager() -> u32 { 7 }

/// Y5: Rust evaluates `source(1)`, then `source(2)`, then `eager()`.
pub fn nested() -> u32 {
    source(1).map_or(source(2).map_or(eager(), |v| v), |v| v)
}

/// Y4: a `move` closure that captures something droppable is an OwnedClosure,
/// which is not callable as a function — and Rust builds it whether or not the
/// branch that calls it runs, and drops it on the branch that does not.
pub fn map_capture(value: Option<u32>, token: Token) -> Option<u32> {
    value.map(move |n| { let m = token.n; drop(token); n + m })
}

pub fn and_then_capture(value: Option<u32>, token: Token) -> Option<u32> {
    value.and_then(move |n| { let m = token.n; drop(token); Some(n + m) })
}

/// A closure that captures nothing droppable stays a plain arrow, called where
/// it stands: there is nothing for the branch that skips it to release.
pub fn filter_capture(value: Option<u32>, token: &Token) -> Option<u32> {
    value.filter(|n| *n > token.n)
}

/// The same combinator with a capture that DOES own something.
pub fn filter_owned(value: Option<u32>, token: Token) -> Option<u32> {
    value.filter(move |n| { let m = token.n; drop(token); *n > m })
}

pub fn is_some_and_owned(value: Option<u32>, token: Token) -> bool {
    value.is_some_and(move |n| { let m = token.n; drop(token); n > m })
}

pub fn map_or_capture(value: Option<u32>, token: Token) -> u32 {
    value.map_or(0, move |n| { let m = token.n; drop(token); n + m })
}

pub fn map_or_else_capture(value: Option<u32>, token: Token, other: Token) -> u32 {
    value.map_or_else(move || other.n, move |n| { let m = token.n; drop(token); n + m })
}

pub fn ok_or_else_capture(value: Option<u32>, token: Token) -> Result<u32, u32> {
    value.ok_or_else(move || { let m = token.n; drop(token); m })
}

/// A closure the source BOUND to a name first, then handed to a combinator.
///
/// The port writes a `move` closure over something with drop glue as an
/// `OwnedClosure`, which is a value and not a bare callable (R10). Asked of the
/// argument's TEXT, a named one read as an ordinary arrow: the emitted `(f)(v)`
/// was a `TypeError` on the first value it saw, and the branch that skips the
/// call walked away from the closure and everything it captured.
pub fn named_closure(value: Option<u32>, token: Token) -> Option<u32> {
    let f = move |n: u32| { let m = token.n; drop(token); n + m };
    value.map(f)
}
