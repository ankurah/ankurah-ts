//! X4: what a consuming loop records about ownership is a FRAME, and it is
//! popped when the loop's body ends.
//!
//! A loop over an owned sequence aliases it into `_seqN` and releases its tail
//! from its own `finally`, so while the body is being written the sequence's
//! NAME belongs to another frame and a refusal inside must leave it alone. That
//! was recorded in a body-global set keyed by the spelling, so a `let xs = ..`
//! written BELOW the loop inherited the answer: the replacement sequence was
//! released by nobody, and a statement that refused above it took the whole
//! vector with it.

pub struct Token {
    pub n: u32,
}

pub fn look(t: &Token) -> u32 {
    t.n
}

/// The shadow: `xs` names the loop's sequence, then names a different one.
pub fn shadowed(xs: Vec<Token>, replacement: Vec<Token>) -> u32 {
    let xs = xs;
    let mut total = 0;
    for item in xs {
        total += look(&item);
    }
    let xs = replacement;
    // The refusal: `BinaryHeap` is a `FromIterator` the port has no
    // construction for, so the hole throws here and `xs` — the REPLACEMENT —
    // is what has to be released as it goes past.
    let _built: std::collections::BinaryHeap<u32> = xs.into_iter().map(|t| t.n).collect();
    total
}

/// Two loops over one spelling, each with its own answer for the element it
/// hands out.
pub fn twice(a: Vec<Token>, b: Vec<Token>) -> u32 {
    let mut total = 0;
    for rest in a {
        total += look(&rest);
    }
    for rest in b {
        total += look(&rest);
    }
    total
}
