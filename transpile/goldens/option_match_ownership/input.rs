//! A `match` on an owned `Option<T>` is written as a null test, because the port
//! writes `Option<T>` as `T | null`. The arm's binding is another name for the
//! same value, and where the match consumes, the arm is what releases it — so
//! the binding has to stand where the `finally` that releases it can see it.
//! A `finally` is a SIBLING of its `try`, and a `const` declared inside the
//! block is not a name it can reach.

pub struct Token {
    pub n: usize,
}

impl Token {
    pub fn new(n: usize) -> Token { Token { n } }
}

pub fn consume(token: Token) -> usize { token.n }

/// The arm keeps what it bound, so the arm releases it.
pub fn read(slot: Option<Token>) -> usize {
    match slot {
        Some(token) => token.n + 1,
        None => 0,
    }
}

/// The arm hands what it bound to a callee, so there is nothing left to
/// release and no block to wrap.
pub fn hand_on(slot: Option<Token>) -> usize {
    match slot {
        Some(token) => consume(token),
        None => 0,
    }
}

/// One arm keeps it and the other hands it on: the release is under a flag.
pub fn either(slot: Option<Token>, keep: bool) -> usize {
    match slot {
        Some(token) => {
            if keep {
                token.n + 100
            } else {
                consume(token)
            }
        }
        None => 0,
    }
}

/// The match reads through a reference and takes nothing, so the caller still
/// owns the value afterwards.
pub fn peek(slot: &Option<Token>) -> usize {
    match slot {
        Some(token) => token.n,
        None => 0,
    }
}
