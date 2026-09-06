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

pub fn take2(a: Token, b: Result<Vec<Token>, String>) -> u32 { a.n + b.map(|v| v.len()).unwrap_or(0) as u32 }

/// R9: the refusal is in the statement's OWN text. `take2` is never entered, so
/// both by-value parameters are still this frame's. There used to be two walks
/// here, and only the one for a refusal in a hoist knew about parameters.
pub fn refused_in_the_text(held: Token, rest: Vec<Token>) -> u32 {
    let _v = take2(held, rest.into_iter().map(pass).collect::<Result<Vec<_>, _>>());
    _v
}

/// S2: a refusal inside a consuming loop. The element THIS turn holds was
/// handed out by the loop and released by nobody: the loop's claim wrote it off
/// as moved, and the tail release starts after the current index.
pub fn refused_in_a_loop(items: Vec<Vec<Token>>) -> u32 {
    let mut total = 0;
    for rest in items {
        let _v = rest.into_iter().map(pass).collect::<Result<Vec<_>, _>>();
        total += 1;
    }
    total
}

pub fn count(xs: Vec<Token>) -> Result<u32, String> { Ok(xs.len() as u32) }

/// S1: the `Vec` is HANDED OVER by an earlier successful `?`, and only then
/// does the statement refuse. The port used to ask the value itself whether it
/// had been moved — a plain array carries no such mark, so the answer was
/// always "nobody has taken it" and the tokens `count` had already released
/// were released a second time. The transfer is this frame's fact, and a flag
/// set where the transfer is written is what records it.
pub fn vec_handed_over_first(rest: Vec<Token>, more: Vec<Token>) -> Result<u32, String> {
    let _pair = (count(rest)?, more.into_iter().map(pass).collect::<Result<Vec<_>, _>>()?);
    Ok(0)
}

/// The same two calls the other way round: the refusal comes FIRST, so `count`
/// is never reached and the `Vec` is still this frame's.
pub fn vec_never_handed_over(rest: Vec<Token>, more: Vec<Token>) -> Result<u32, String> {
    let _pair = (more.into_iter().map(pass).collect::<Result<Vec<_>, _>>()?, count(rest)?);
    Ok(0)
}
