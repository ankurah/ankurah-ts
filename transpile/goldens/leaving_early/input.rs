//! Three ways a statement leaves before it reads what it is holding.
//!
//! A `?` is hoisted above the statement that uses it, so the emitted text
//! reaches everything between the hoist and the read still holding the value.
//! What can happen in between decides whether anything owes a release, and two
//! of the three answers were missing.
//!
//! An `await` is one: a rejected promise leaves through it with the wrapper
//! still in hand, and Rust drops that temporary on the unwind. A second `?` in
//! the same statement is another — and where the `?` is on an `Option`, the
//! temporary IS the payload rather than a wrapper, so it is released from a
//! flag this frame declares rather than from the runtime's own mark, which an
//! array does not carry.
//!
//! And `count` is the third thing that was holding something: it drains the
//! iterator to the end, so every element it walked past is dropped, including
//! the survivors an owning adaptor below it kept.

pub struct Token {
    pub n: u32,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

pub fn passr(t: Token) -> Result<Token, u32> {
    Ok(t)
}

pub fn eat(v: Vec<Token>) -> u32 {
    v.len() as u32
}

pub fn maybe_tokens(flag: bool) -> Option<Vec<Token>> {
    match flag {
        true => Some(vec![Token { n: 1 }, Token { n: 2 }]),
        false => None,
    }
}

/// The `?` runs, then the await rejects, and the wrapper is still holding the
/// token.
pub async fn awaited(t: Token, f: impl std::future::Future<Output = u32>) -> Result<(u32, Token), u32> {
    Ok((f.await, passr(t)?))
}

/// The first `?` succeeds and the second leaves, with the first payload still
/// in hand.
pub fn two_payloads(a: bool, b: bool) -> Option<u32> {
    Some(eat(maybe_tokens(a)?) + eat(maybe_tokens(b)?))
}

/// `count` drains the whole sequence, and the adaptor's survivors go with it.
pub fn counted(tokens: Vec<Token>) -> usize {
    tokens.into_iter().filter(|t| t.n > 1).count()
}
