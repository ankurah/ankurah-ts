//! I4: what a refused statement owns by the time it refuses.

pub struct Token { pub n: u32 }
impl Token { pub fn new(n: u32) -> Token { Token { n } } }
impl Drop for Token { fn drop(&mut self) {} }

pub fn pass(t: Token) -> Result<Token, String> { Ok(t) }

/// The first operand is evaluated and its `?` checked; the second refuses. The
/// temporary holding the first, and the sequence the second was walking, are
/// both live when the hole throws.
pub fn nested(first: Token, rest: Vec<Token>) -> Result<u32, String> {
    let _pair = (pass(first)?, rest.into_iter().map(pass).collect::<Result<Vec<_>, _>>()?);
    Ok(_pair.0.n)
}

/// The same refusal alone: nothing ran before it, and the sequence it was
/// walking is still the block's.
pub fn only_refused(rest: Vec<Token>) -> Result<u32, String> {
    let _v = rest.into_iter().map(pass).collect::<Result<Vec<_>, _>>()?;
    Ok(0)
}

/// A local the statement moves into a call that is NOT refused, with the
/// refusal beside it.
pub fn moved_then_refused(held: Token, rest: Vec<Token>) -> Result<u32, String> {
    let _v = (take(held), rest.into_iter().map(pass).collect::<Result<Vec<_>, _>>()?);
    Ok(0)
}
pub fn take(t: Token) -> u32 { t.n }
