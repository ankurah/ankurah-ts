//! What a call HANDS OVER is a local of the body it hands it to. O1, L3.
//!
//! For: Rust drops a by-value closure parameter at the end of every invocation
//! — on the normal return and while an unwind passes through — because it is a
//! local of the closure's body, exactly as a function's parameter is a local of
//! the function's. The port released none of them: `position`'s callback, which
//! is handed the element BY VALUE, stored it nowhere and dropped nothing, so
//! every element the callback was given leaked. The TERMINAL cannot make that
//! good, because a legal callback may transfer the element somewhere else and
//! the terminal cannot see which.
//!
//! The same rule reaches a `match` arm: a name an arm binds by value is a local
//! of that arm, whatever the SUBJECT's shape. The claim used to be made only
//! where the pattern bound the whole subject, so a tuple subject whose arms
//! bind both operands released neither (`storage-common`'s two comparators,
//! six arms).

pub struct Token(pub u32);

impl Drop for Token {
    fn drop(&mut self) {}
}

pub struct Holder {
    pub item: Token,
    pub tag: u32,
}

/// `position`'s closure takes the element BY VALUE and stores it nowhere.
pub fn position_of(tokens: Vec<Token>, want: u32) -> Option<usize> {
    tokens.into_iter().position(|token| token.0 == want)
}

/// The same closure leaving through a throw: Rust's unwind drops what the
/// invocation was handed.
pub fn position_or_fail(tokens: Vec<Token>, bad: u32) -> Option<usize> {
    tokens.into_iter().position(|token| {
        if token.0 == bad {
            panic!("bad token");
        }
        false
    })
}

/// A closure that hands its parameter ON releases nothing: `b` goes to `drop`
/// and `a` is the closure's answer.
pub fn first_kept(tokens: Vec<Token>) -> Option<Token> {
    tokens.into_iter().reduce(|a, b| {
        drop(b);
        a
    })
}

/// A closure's expression body is its VALUE, so a field read in it is a partial
/// move: `item` goes to the caller and the rest of `holder` is dropped here.
pub fn items(holders: Vec<Holder>) -> Vec<Token> {
    holders.into_iter().map(|holder| holder.item).collect()
}

/// A BORROWED chain hands nothing over: `t` is a `&Token` and the sequence is
/// still the caller's.
pub fn find_borrowed(tokens: &Vec<Token>, want: u32) -> Option<&Token> {
    tokens.iter().find(|t| t.0 == want)
}

/// L3: a tuple subject whose arms bind BOTH operands by value. Neither binding
/// is the whole subject, so neither used to be released.
pub fn total(a: Token, b: Token) -> u32 {
    match (a, b) {
        (x, y) => x.0 + y.0,
    }
}

/// An arm that hands its binding on releases it nowhere, and still owes the
/// position its pattern did not name.
pub fn keep_first(a: Option<Token>, b: Option<Token>) -> Option<Token> {
    match (a, b) {
        (Some(x), _) => Some(x),
        (None, other) => other,
    }
}
