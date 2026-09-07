//! What a refusal owes for the statements BELOW it.
//!
//! A refused statement throws, so every statement after it in the block is
//! never reached — and a value one of them was going to hand away is still
//! this frame's. The refusal already released what IT was going to hand away;
//! it now releases those too.
//!
//! The loop is the shape that made it visible. `for rest in b` takes the
//! element out of the sequence, and the loop's tail release starts after the
//! current index: an element the turn was handed and never passed on is
//! reachable by nobody else.

use std::collections::BinaryHeap;

pub struct Token {
    pub n: i64,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

pub fn take(t: Token) -> i64 {
    t.n
}

/// The element is handed out, the refusal throws, and the turn still owns it.
pub fn in_a_loop(b: Vec<Token>) -> i64 {
    let mut total = 0i64;
    for rest in b {
        let h: BinaryHeap<i64> = vec![1i64].into_iter().collect();
        let _ = h;
        total += take(rest);
    }
    total
}

/// The same for a by-value parameter a later statement would have moved.
pub fn a_parameter(t: Token) -> i64 {
    let h: BinaryHeap<i64> = vec![1i64].into_iter().collect();
    let _ = h;
    take(t)
}
