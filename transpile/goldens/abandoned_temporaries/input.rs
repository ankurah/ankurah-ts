//! R0(3): a `?`'s temporary is released however its statement is left.
//!
//! `Ok((take(a)?, take(b)?))` reaches the first `?`'s `unwrap` only when the
//! second one succeeds. When the second returns `Err`, and when something below
//! it panics, Rust drops the first temporary on the way out; the port used to
//! abandon it, because "the `unwrap` that follows consumes the wrapper" is a
//! complete answer only where that `unwrap` runs. Fifty-one of ankql's leak
//! reports were this one fact.

pub struct Token {
    pub n: u32,
}

pub struct Refused;

pub fn take(n: u32) -> Result<Token, Refused> {
    if n == 0 {
        return Err(Refused);
    }
    Ok(Token { n })
}

/// Two `?` in one expression. The first one's `Token` is owned by nobody once
/// the second leaves through its own `return`.
pub fn both(a: u32, b: u32) -> Result<(Token, Token), Refused> {
    Ok((take(a)?, take(b)?))
}

/// The same statement written as a `let`, where each `?` names its own
/// temporary. The existing temporaries machinery already owns those, so the
/// wrapper needs no release of its own and none is written: the rule fires
/// where nothing else has taken the value, not everywhere a `?` stands.
pub fn three(a: u32, b: u32, c: u32) -> Result<u32, Refused> {
    let sum = take(a)?.n + take(b)?.n + take(c)?.n;
    Ok(sum)
}

/// The second `?`'s operand PANICS before the `?` is reached. Rust unwinds and
/// drops the first temporary on the way out.
pub fn both_or_panic(a: u32, b: u32) -> Result<(Token, Token), Refused> {
    Ok((take(a)?, exploding(b)?))
}

pub fn exploding(n: u32) -> Result<Token, Refused> {
    if n == 99 {
        panic!("exploding was asked for 99");
    }
    take(n)
}

/// One `?` whose `unwrap` stands alone in the statement. Nothing runs between
/// the two, so no release is written at all.
pub fn only_one(a: u32) -> Result<u32, Refused> {
    let t = take(a)?;
    Ok(t.n)
}
