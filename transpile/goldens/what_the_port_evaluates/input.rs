//! The evaluation plan is made from what the PORT writes, and in the order
//! RUST evaluates.
//!
//! Two ways the plan was made from the wrong thing. A place the port writes as
//! a CALL — `handle.n` on a value behind a `Deref` becomes `handle.deref().n`,
//! and `deref()` on a value somebody dropped throws — was read as a place, so
//! the move beside it was called unconditional. And a struct literal's fields
//! were translated in DECLARATION order while Rust evaluates them in the order
//! the literal writes them, so a value moved into a later-declared field was
//! handed over before the field Rust runs first had thrown.
//!
//! The assignment below is the third: a local a branch may already have handed
//! away is given a new value here, so the old one is released under its flag
//! and the flag goes back to false.

pub struct Token {
    pub n: i64,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

pub struct Inner {
    pub n: i64,
}

pub struct Handle {
    inner: Inner,
}

impl Handle {
    pub fn new(n: i64) -> Handle {
        Handle { inner: Inner { n } }
    }
}

impl std::ops::Deref for Handle {
    type Target = Inner;
    fn deref(&self) -> &Inner {
        &self.inner
    }
}

pub struct Event {
    pub t: Token,
    pub n: i64,
}

pub struct Reordered {
    pub token: Token,
    pub n: i64,
}

pub fn mk(n: i64) -> Token {
    Token { n }
}

/// The port writes `handle.deref().n` where the source wrote a place.
pub fn through_a_deref(token: Token, handle: Handle) -> Event {
    Event { t: token, n: handle.n }
}

/// Rust evaluates `n` first and moves `token` only if it returned.
pub fn reordered(token: Token, value: Option<i64>) -> Reordered {
    Reordered { n: value.unwrap(), token }
}

/// The old value is released where the assignment stands, and the flag that
/// said it had been handed away goes back with it.
pub fn reassigned(low: Token, again: bool, value: Option<i64>) -> i64 {
    let mut low = low;
    if again {
        low = mk(2);
    }
    let built = Reordered { n: value.unwrap(), token: low };
    built.n
}
