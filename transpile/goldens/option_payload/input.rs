//! A consuming `Option` match whose arms test inside the payload. Q3.
//!
//! For: `Option<T>` is `T | null` here, so the value under test IS the payload
//! — there is no wrapper to take a name out of and none left to release. The
//! arm chain the port writes for an `Option` claims only the names each arm
//! bound, so `Some(Value::Held(token)) => sink(token)` handed `token` on and
//! left the `Value` it came out of owned by nobody, while the name it bound was
//! never detached from that value and was released twice. It was a hole.
//!
//! The lowering it wanted is the one the port already writes for an enum whose
//! arms take its payload: `intoMatch`, which detaches what the arm binds and
//! marks the value moved. What was missing is the rewriting — the arms have to
//! partition into a run of `Some(P)` arms and the one that stands for `None` —
//! and the payload's own TYPE, because the subject expression still resolves to
//! the `Option` around it.
//!
//! A bare `_` is NOT that shape: it stands for every value the arms above it
//! left, `Some` cases included, so it belongs to the enum match and to the null
//! test's else at once. That one keeps the hole.

pub struct Token(pub u32);
impl Drop for Token { fn drop(&mut self) {} }

pub enum Value {
    Held(Token),
    Text(String),
    Empty,
}

pub fn sink(t: Token) -> u32 { t.0 }

/// The corpus shape: `Some(Value::X(y))` arms, a `Some(other)` catch-all and a
/// `None`.
pub fn read(value: Option<Value>) -> u32 {
    match value {
        Some(Value::Held(token)) => sink(token),
        Some(Value::Text(s)) => s.len() as u32,
        Some(other) => hold(other),
        None => 0,
    }
}

pub fn hold(v: Value) -> u32 { 7 }

/// The same with no catch-all.
pub fn read_exact(value: Option<Value>) -> u32 {
    match value {
        Some(Value::Held(token)) => sink(token),
        Some(Value::Text(s)) => s.len() as u32,
        Some(Value::Empty) => 1,
        None => 0,
    }
}

/// A BORROWED option match is unchanged: nothing is taken apart.
pub fn peek(value: &Option<Value>) -> u32 {
    match value {
        Some(Value::Held(_)) => 1,
        Some(_) => 2,
        None => 0,
    }
}

/// The shape that is still a hole: `_` covers the `Some` cases no arm named as
/// well as `None`.
pub fn read_loosely(value: Option<Value>) -> u32 {
    match value {
        Some(Value::Held(token)) => sink(token),
        _ => 0,
    }
}
