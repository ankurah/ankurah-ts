//! A guard on an arm: the second test, made after the pattern's and reading the
//! names the pattern bound.
//!
//! The runtime's `match` and `intoMatch` dispatch on the variant alone, so a
//! guard cannot be part of the key, and an arm whose guard fails has to reach
//! the arm BELOW it. Before the sixth pass the port reported "the guard is
//! dropped" and ran the arm unconditionally, which is runnable wrong code:
//! `core/src/node.rs:621` answered the event-bridge path for an EMPTY bridge,
//! and `core/src/context.rs:187` lost `Err(NoDurablePeers) if cached => ()`
//! entirely, so a cached entity with no durable peers answered an error.
//!
//! What is written instead: the arm's pattern test opens a block, the names it
//! binds stand in that block, the guard is tested there, and an arm that ran
//! leaves the chain. A `Result` side reads its payload ONCE and tries its arms
//! against that name.

pub struct Token { pub n: u32 }
impl Token { pub fn new(n: u32) -> Token { Token { n } } }

pub enum Guarded { Same(Token, bool), Other }

/// A guard on a CONSUMING enum match; the arm below it takes the same variant.
pub fn guarded_consuming(input: Guarded) -> u32 {
    match input {
        Guarded::Same(token, true) if token.n > 0 => { drop(token); 1 }
        Guarded::Same(token, _) => { drop(token); 2 }
        rest => { drop(rest); 0 }
    }
}

pub enum Weight { Light(u32), Heavy(u32) }

/// A guard on a BORROWED enum match, with a catch-all below it.
pub fn heaviest(w: &Weight) -> u32 {
    match w {
        Weight::Light(n) if *n > 10 => 10,
        Weight::Light(n) => *n,
        _ => 99,
    }
}

pub enum Refusal { Empty, Late }

/// The `Result` shape of core/src/context.rs:187: a guarded Err arm above the
/// catch-all Err arm. The guarded arm used to vanish entirely.
pub fn settle(r: Result<u32, Refusal>, cached: bool) -> Result<u32, Refusal> {
    match r {
        Ok(n) => Ok(n),
        Err(Refusal::Empty) if cached => Ok(0),
        Err(e) => Err(e),
    }
}

/// The `Result` shape of core/src/node.rs:621: a guarded Ok arm and a catch-all,
/// over a call's result rather than a parameter, which is how the corpus writes
/// it (`match self.collect_event_bridge(..).await { .. }`).
pub fn collect(n: u32) -> Result<Vec<u32>, Refusal> {
    if n == 0 { Ok(Vec::new()) } else { Ok(vec![n]) }
}

pub fn bridge(n: u32) -> u32 {
    match collect(n) {
        Ok(events) if !events.is_empty() => events.len() as u32,
        _ => 0,
    }
}

/// A guard that reads a name the pattern bound, in a value position.
pub fn describe(w: Weight) -> String {
    match w {
        Weight::Light(n) if n == 0 => "nothing".to_string(),
        Weight::Light(n) => format!("light {}", n),
        Weight::Heavy(n) => format!("heavy {}", n),
    }
}

pub struct Detail { pub why: String }

pub enum Rich { Empty(Detail), Late(Detail) }

/// An arm whose inner pattern both TESTS and BINDS takes part of the payload
/// out and leaves the rest, and the port has no way to release an object minus
/// one field: R12 puts the refusal in the BRANCH, so a value the pattern does
/// not match still reaches the arm below it.
pub fn settle_rich(r: Result<u32, Rich>, cached: bool) -> Result<u32, Rich> {
    match r {
        Ok(n) => Ok(n),
        Err(Rich::Empty(d)) if cached => { drop(d); Ok(0) }
        Err(e) => Err(e),
    }
}

/// A guarded arm whose body neither returns nor throws: the chain has to stop
/// the arms below it running.
pub fn count(w: &Weight, into: &mut Vec<u32>) {
    match w {
        Weight::Light(n) if *n > 3 => { into.push(*n); }
        Weight::Light(_) => { into.push(0); }
        Weight::Heavy(n) => { into.push(*n * 2); }
    }
}
