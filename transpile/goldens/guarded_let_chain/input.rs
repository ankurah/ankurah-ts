//! `if let PAT = e && guard { A } else { B }`, where the guard is false.
//!
//! Rust runs B and drops what the pattern took on the way. Written as a nested
//! `if` with no `else` of its own, the guard-false path fell out of BOTH
//! branches: the function answered `undefined` where the source answers B, and
//! the payload the pattern had already taken out was owned by nobody.
//!
//! The three subjects that reach the if-chain are here — the runtime's
//! `Result`, a nullable, and a plain struct field read behind a test — because
//! each takes its payload a different way and each has to release it the same
//! way.

pub struct Token {
    pub n: i64,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

/// `unwrap()` has already taken the payload by the time the guard is made.
pub fn from_a_result(r: Result<Token, i64>, allow: bool) -> i64 {
    if let Ok(token) = r && allow {
        token.n
    } else {
        7
    }
}

/// A nullable IS its payload, so the binding is the subject itself.
pub fn from_an_option(o: Option<Token>, allow: bool) -> i64 {
    if let Some(token) = o && allow {
        token.n
    } else {
        7
    }
}

/// And a guard that succeeds still hands the payload to the branch.
pub fn allowed(o: Option<Token>) -> i64 {
    if let Some(token) = o && token.n > 0 {
        token.n
    } else {
        7
    }
}
