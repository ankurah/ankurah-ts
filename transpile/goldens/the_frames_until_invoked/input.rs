//! What the frame still owns while a statement is being evaluated.
//!
//! Rust drops what a statement has moved and what it has BUILT while it
//! unwinds, so every one of these is the frame's until the call it was
//! evaluated for is invoked. Four ways the rule was not reached:
//!
//! a `?` standing after a move — `is_place` answers yes for one, which is
//! right for "does this leave a value to release" and wrong for "can this
//! leave the frame"; a tuple or array ELEMENT, whose emitter had no lift at
//! all; a move NESTED inside an operand, which the walk reached with a
//! straight-line position; and a droppable TEMPORARY in a statement that moves
//! no name, which the rule declined to look at.

pub struct Token {
    pub n: i64,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

pub struct Box3 {
    pub a: Option<Token>,
    pub k: Token,
    pub c: i64,
}

pub struct Box4 {
    pub a: Token,
    pub b: Token,
    pub c: i64,
}

pub fn mk(n: i64) -> Token {
    Token { n }
}

pub fn take2(a: Token, b: i64) -> i64 {
    a.n + b
}

/// The `?` leaves before the call it was evaluated for is invoked.
pub fn after_a_question(t: Token, o: Option<i64>) -> Option<i64> {
    Some(take2(t, o?))
}

/// The element written as the move, and a later element that throws.
pub fn in_a_tuple(token: Token, o: Option<i64>) -> (Token, i64) {
    (token, o.unwrap())
}

/// The move nested inside the first field, and a later field that throws.
pub fn nested_in_an_operand(x: Token, k: Token, o: Option<i64>) -> Box3 {
    Box3 { a: Some(x), k, c: o.unwrap() }
}

/// Two built values and no moved name anywhere in the statement.
pub fn only_temporaries(o: Option<i64>) -> Box4 {
    Box4 { a: mk(1), b: mk(2), c: o.unwrap() }
}
