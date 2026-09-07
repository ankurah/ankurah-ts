//! Leg A at a CALL BOUNDARY: one decision about what the port holds as a
//! cursor, read by everything that has to agree with it.
//!
//! A body that takes `I: Iterator<Item = V>` is handed a `SeqCursor`, because
//! that is what an `into_iter()` on a bounded parameter builds. Three things
//! follow, and each was wrong on its own before this golden was written. A
//! CONCRETE caller writes its `into_iter()` as an array, so the array is
//! wrapped where it crosses into the cursor — handed over as it stood,
//! `walk.next()` inside the callee called a method an array has not got. A
//! by-value cursor parameter is the body's, and Rust drops it at the end,
//! releasing whatever the walk never reached. And a cursor that has given up
//! its rest is an ARRAY from that point on, so `count` is the array's `length`
//! rather than a method no array declares.

pub struct Token {
    pub n: i64,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

/// Takes one element by hand. Everything the walk did not reach is still the
/// cursor's, and the cursor is this body's to drop.
pub fn first<I>(mut walk: I) -> Option<Token>
where
    I: Iterator<Item = Token>,
{
    walk.next()
}

/// Never advances the walk at all: the whole sequence goes with the cursor.
pub fn ignored<I>(_walk: I) -> i64
where
    I: Iterator<Item = Token>,
{
    7
}

/// A terminal the array's own table answers, once the cursor has given up its
/// rest: `lastOwned` releases everything it walked past, which is what Rust's
/// `Iterator::last` does.
pub fn last_of<I>(walk: I) -> Option<Token>
where
    I: Iterator<Item = Token>,
{
    walk.last()
}

/// And an eager adaptor, whose owned helper releases the prefix it skipped.
pub fn rest_after<I>(walk: I, n: usize) -> Vec<Token>
where
    I: Iterator<Item = Token>,
{
    walk.skip(n).collect()
}

pub fn head(tokens: Vec<Token>) -> Option<Token> {
    first(tokens.into_iter())
}

pub fn dropped(tokens: Vec<Token>) -> i64 {
    ignored(tokens.into_iter())
}

pub fn tail(tokens: Vec<Token>) -> Option<Token> {
    last_of(tokens.into_iter())
}

pub fn all_but(tokens: Vec<Token>, n: usize) -> Vec<Token> {
    rest_after(tokens.into_iter(), n)
}

/// A generic caller already holds a cursor, and hands it over as it stands.
pub fn head_of<J>(walk: J) -> Option<Token>
where
    J: Iterator<Item = Token>,
{
    first(walk)
}
