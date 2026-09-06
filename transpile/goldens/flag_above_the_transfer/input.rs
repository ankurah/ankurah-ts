//! U3: a move flag stands immediately above the transfer it reports.
//!
//! A `?` evaluates the consuming call inside its own hoist and leaves the
//! statement on the error path, so its flag cannot stand below the prelude —
//! it would never be set and the block would release what the callee took.
//! It used to stand above the WHOLE prelude instead, which is above the
//! arguments the port lifts out precisely so that the flag can stand below
//! them, and above every earlier `?` in the same statement. Both of those are
//! places the statement can still leave, with the value handed to nobody and
//! a flag saying otherwise.

pub struct Token {
    pub n: u32,
}

pub struct Refused;

/// A gate whose `?` gives the block an exit BEFORE the transfer, which is what
/// makes the local's drop conditional and gives it a flag at all.
pub fn gate(open: bool) -> Result<u32, Refused> {
    if open {
        Ok(1)
    } else {
        Err(Refused)
    }
}

/// Built where the call stands, so the port lifts it above the move flag.
pub fn build(explode: bool) -> Token {
    if explode {
        panic!("build exploded");
    }
    Token { n: 4 }
}

pub fn eat(a: Token, b: Token) -> Result<u32, Refused> {
    Ok(a.n + b.n)
}

/// The callee takes the token and releases it on its own early return, so the
/// caller's flag has to be set even though the statement leaves through the
/// `?` — this is the shape the flag stood above the prelude for.
pub fn consume(t: Token, fail: bool) -> Result<u32, Refused> {
    if fail {
        return Err(Refused);
    }
    Ok(t.n)
}

/// An argument lifted above the flag. `build(true)` panics after the flag used
/// to be set, leaving `held` released by nobody.
pub fn lifted(explode: bool) -> Result<u32, Refused> {
    let held = Token { n: 1 };
    let opened = gate(true)?;
    let total = eat(build(explode), held)?;
    Ok(total + opened)
}

/// Two `?` operands, each handing away a local of its own. The second one's
/// flag used to be set above the FIRST one's call, so an `Err` from the first
/// left `second` flagged as handed over and released by nobody.
pub fn two_transfers(fail: bool) -> Result<u32, Refused> {
    let first = Token { n: 1 };
    let second = Token { n: 2 };
    let opened = gate(true)?;
    let total = consume(first, fail)? + consume(second, false)?;
    Ok(total + opened)
}

/// One `?` whose own operand performs the transfer: the flag still stands
/// above the hoist, because the statement's own text is never reached on the
/// error path and the callee has already released what it took.
pub fn one_transfer(fail: bool) -> Result<u32, Refused> {
    let held = Token { n: 3 };
    let opened = gate(true)?;
    let total = consume(held, fail)?;
    Ok(total + opened)
}
