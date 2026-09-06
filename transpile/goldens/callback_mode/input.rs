//! How a terminal took its callback, and what `by_ref` names. O2, O5.
//!
//! For: Rust's terminals take their `F` BY VALUE and drop it where the call
//! ends, so `xs.into_iter().find(p)` consumes `p` and everything it captured.
//! `find(&mut p)` type-checks through the `impl FnMut for &mut F` blanket: what
//! the terminal takes by value is then the REFERENCE, dropping a reference does
//! nothing, and `p` is still the caller's to call again. The port has no
//! reference to hand over — the closure object itself goes — so every helper
//! released it whatever the source wrote, and the call after the terminal read
//! captures that were gone while the block's own release dropped it a second
//! time.
//!
//! `by_ref` is the same question about the SEQUENCE. `Iterator::by_ref(&mut
//! self) -> &mut Self` is a borrowed view of the iterator it was called on, so
//! the chain below it advances that iterator and leaves the rest of it in the
//! named place. Written as the camelCase of its Rust name it was `it.byRef()`,
//! a method no array declares, and the check that refuses a consuming terminal
//! on a named iterator never saw the name.

pub struct Token(pub u32);

impl Drop for Token {
    fn drop(&mut self) {}
}

/// `F` by value: the terminal is what releases the closure, and releasing it
/// releases what it captured.
pub fn find_owning(tokens: Vec<Token>, want: Token) -> Option<Token> {
    tokens.into_iter().find(move |t| t.0 == want.0)
}

/// `&mut F`: the closure is the caller's, is called again after the terminal,
/// and is released by the block that declared it.
pub fn find_borrowing(tokens: Vec<Token>, want: Token) -> Option<Token> {
    let mut p = move |t: &Token| t.0 == want.0;
    let found = tokens.into_iter().find(&mut p);
    drop(p);
    found
}

/// The same on the READING family, which releases its callback too.
pub fn read_borrowing(tokens: &Vec<Token>, want: Token) -> usize {
    let p = move |t: &&Token| t.0 == want.0;
    let mut hits = 0;
    if tokens.iter().find(&p).is_some() {
        hits += 1;
    }
    if tokens.iter().find(&p).is_some() {
        hits += 1;
    }
    drop(p);
    hits
}

/// O5: a consuming terminal reached through `by_ref` names the iterator, and is
/// refused exactly as `(&mut it).find(..)` is.
pub fn through_by_ref(tokens: Vec<Token>) -> Option<Token> {
    let mut it = tokens.into_iter();
    it.by_ref().find(|t| t.0 > 0)
}

/// And on a BORROWED chain `by_ref` is the identity: nothing is consumed.
pub fn borrowed_through_by_ref(tokens: &Vec<Token>) -> Option<&Token> {
    let mut it = tokens.iter();
    it.by_ref().find(|t| t.0 > 0)
}
