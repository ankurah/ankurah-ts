//! Which of two things a hole is: the statement's OWN evaluation, or a CALLABLE
//! the statement passes.
//!
//! A hole in the statement's own evaluation stops the statement, so everything
//! it was going to hand away is still this frame's and the frame releases it. A
//! hole inside a callable the statement PASSES did not: the call that received
//! the callback ran, took what it takes, and invoked the callback — a throw out
//! of a call that already happened.
//!
//! Read off the rendered text — an arrow before the first `unsupported(` — the
//! emitter's OWN spelling answered it. A block expression is written as an
//! immediately-invoked `(() => { .. })()`, so every statement holding one had
//! its cleanup suppressed and leaked whatever it still owned; so did a
//! statement with `"=>"` in a string, and one with an unrelated closure
//! argument beside a hole in its own evaluation. The LOWERING knows which,
//! because it knows when it is inside a callable's body.

use std::collections::BinaryHeap;

pub struct Token {
    pub n: u32,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

pub fn take2(a: Token, b: u32) -> u32 {
    let n = a.n;
    n + b
}

pub fn apply<F>(f: F) -> u32
where
    F: Fn(u32) -> u32,
{
    f(1)
}

/// A hole the port writes inside its own IIFE, in the statement's evaluation.
pub fn block_expression(held: Token, rest: Vec<u32>) -> u32 {
    ({
        let _h: BinaryHeap<u32> = rest.into_iter().collect();
        0u32
    }) + take2(held, 1)
}

/// An arrow inside a STRING.
pub fn arrow_in_a_string(held: Token, rest: Vec<u32>) -> u32 {
    "=>".len() as u32
        + ({
            let _h: BinaryHeap<u32> = rest.into_iter().collect();
            0u32
        })
        + take2(held, 1)
}

/// An unrelated closure argument standing beside the hole.
pub fn beside_a_closure(held: Token, rest: Vec<u32>) -> u32 {
    apply(|x| x + 1)
        + ({
            let _h: BinaryHeap<u32> = rest.into_iter().collect();
            0u32
        })
        + take2(held, 1)
}

/// And the rule the callable test exists for: the hole is inside a callable
/// this statement passes, and `take2` has already taken `held`.
pub fn inside_a_callable(held: Token) -> u32 {
    take2(held, 1)
        + apply(|x| {
            let _h: BinaryHeap<u32> = vec![x].into_iter().collect();
            0u32
        })
}
