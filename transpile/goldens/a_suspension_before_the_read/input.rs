// A suspension while a `?`'s wrapper is still in hand.
//
// A `?` is hoisted ABOVE the statement it stands in, so its wrapper is live
// from the top of it. An `await` that suspends after that hoist and before the
// wrapper's first mention can REJECT and leave the statement with the wrapper
// still holding its `Ok` payload — which Rust drops while it unwinds.
//
// Which awaits those are is a fact about the lowering: where the await's own
// operand ends, against where the `?` stands. In `pair(f.await, eat(pass(t)?))`
// the awaiting finishes before the `?`; in `pair(0, step(pass(n)?).await)` it
// finishes after it, because the awaited operand is what the `?` helped to
// build. Read as brackets in the RENDERED text, the call around the wrapper's
// first mention left an opening `(` and the rule answered "the mention stands
// inside the awaited operand", so no guard was written at all.

pub struct Token {
    pub n: i64,
}

impl Drop for Token {
    fn drop(&mut self) {}
}

pub fn pass(t: Token) -> Result<Token, String> {
    Ok(t)
}

pub fn eat(t: Token) -> i64 {
    t.n
}

pub fn pair(a: i64, b: i64) -> i64 {
    a + b
}

pub fn pass_n(n: i64) -> Result<i64, String> {
    Ok(n)
}

pub async fn step(n: i64) -> i64 {
    n
}

/// The awaiting finishes BEFORE the `?`, and the wrapper's first mention is
/// inside a later call. A rejected promise leaves through the await with the
/// wrapper still holding its payload.
pub async fn suspends_first(t: Token, f: impl std::future::Future<Output = i64>) -> Result<i64, String> {
    Ok(pair(f.await, eat(pass(t)?)))
}

/// The awaiting finishes AFTER the `?`, because the awaited operand is what the
/// `?` built: the wrapper is consumed before anything suspends.
pub async fn suspends_last(n: i64) -> Result<i64, String> {
    Ok(pair(0, step(pass_n(n)?).await))
}
