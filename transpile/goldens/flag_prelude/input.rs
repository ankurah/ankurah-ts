//! Where a move FLAG stands, and what has to stand above it. O6, N1, N2, N3.
//!
//! For: a drop flag says "somebody else owns this now", and the block's
//! `finally` reads it. Written above everything the statement evaluates, it was
//! set before the values the call needs had been built — so an argument that
//! THREW left the flag saying the value had been handed over while the callee
//! had never been reached, and nothing released it. The flag belongs
//! immediately above the TRANSFER: below the receiver, below every argument
//! that can throw, and inside whatever branch, arm or chain the call is written
//! in. `?` is the one shape whose transfer happens in the lift itself, and its
//! flag stands above the lift as every flag used to.
//!
//! An argument lifted for this is evaluated earlier than Rust evaluates it, so
//! if a later one throws, nobody owns what the lift produced: it is released
//! however the expression is left, asked of the runtime first because the call
//! it was lifted for may have consumed it.

pub struct Token(pub u32);

impl Drop for Token {
    fn drop(&mut self) {}
}

pub struct Held {
    pub n: u32,
}

pub struct Sink;

impl Sink {
    pub fn swallow(&self, t: Token, n: u32) -> u32 {
        n
    }
}

pub fn eat(t: Token, n: u32) -> u32 {
    n
}

pub fn eat_two(t: Token, u: Token, n: u32) -> u32 {
    n
}

pub fn make() -> Token {
    Token(9)
}

pub fn boom(fail: bool) -> Held {
    if fail {
        panic!("boom");
    }
    Held { n: 1 }
}

/// N1: a FIELD of a call. `is_place` answers "yes, a place" for any
/// `Expr::Field` without looking at what it is read out of.
pub fn field_of_call(c: Token, fail: bool, skip: bool) -> u32 {
    if skip {
        return 0;
    }
    eat(c, boom(fail).n)
}

/// N1: an INDEX whose index is a call.
pub fn index_of_call(c: Token, xs: Vec<u32>, fail: bool, skip: bool) -> u32 {
    if skip {
        return 0;
    }
    eat(c, xs[which(fail)])
}

pub fn which(fail: bool) -> usize {
    if fail {
        panic!("boom");
    }
    0
}

/// O6: the RECEIVER can throw, and it was never lifted at all.
pub fn throwing_receiver(sink: Option<Sink>, c: Token, skip: bool) -> u32 {
    if skip {
        return 0;
    }
    sink.unwrap().swallow(c, 1)
}

/// O6: the call stands inside a branch, where nothing used to be lifted —
/// and where this arm wrote no flag at all, so the callee took `c` and the
/// block released it a second time.
pub fn inside_a_branch(c: Token, o: Option<u32>, skip: bool) -> u32 {
    if skip {
        return 0;
    }
    match o {
        Some(n) => eat(c, n + 1),
        None => 0,
    }
}

/// N3: the first lifted argument holds a value Rust had not yet built. If the
/// second one throws, nobody owns it.
pub fn two_lifts(c: Token, o: Option<u32>, skip: bool) -> u32 {
    if skip {
        return 0;
    }
    eat_two(c, make(), o.unwrap())
}

/// A `?` lifts the very call that consumes, and leaves the statement on the
/// error path: its flag stands ABOVE the lift.
pub fn through_try(c: Token, fail: bool, skip: bool) -> Result<u32, String> {
    if skip {
        return Ok(0);
    }
    let n = give(c, fail)?;
    Ok(n)
}

pub fn give(t: Token, fail: bool) -> Result<u32, String> {
    if fail {
        return Err("no".to_string());
    }
    Ok(t.0)
}
