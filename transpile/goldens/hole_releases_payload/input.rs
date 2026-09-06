//! A HOLE in a consuming arm releases what the arm was handed.
//!
//! R12 says a site whose emission is KNOWN wrong throws where the branch would
//! have run. It does not say the branch may abandon what it was given.
//! `intoMatch` marks the subject moved and hands the payload to the arm, and
//! releases nothing of its own on any path out — so an arm that throws still
//! owns the whole payload, and a refusal that walked away from it turned a
//! reported gap into a leak: the `Inner`, both its `Token`s and the trailing
//! `Token` were all collected with nobody having dropped them.
//!
//! J4 is the same rule at a CALL. A call the engine refuses is replaced by a
//! hole, which throws before anything the call would have consumed reaches a
//! new owner — so the receiver and every argument are still the block's, and
//! counting them as moved left the block releasing nothing. `finish` below is
//! that shape: a `map.entry(..)` finisher on a receiver the engine could not
//! type, holding two owned values Rust would have moved into the map.

pub struct Token {
    pub n: u32,
}

impl Token {
    pub fn new(n: u32) -> Token { Token { n } }
}

pub enum Inner { A((Token, Token)), B((Token, Token)) }

pub enum Wrap { Held(Inner, Token), Empty }

/// The alternatives take their names out of a tuple, whose destructuring is not
/// a name or a field list, so the translator cannot read the binding back. The
/// TEST is still written — a value neither alternative matches reaches the arm
/// below, which is D2's rule — and the branch releases the payload before it
/// refuses.
pub fn pick(w: Wrap) -> u32 {
    match w {
        Wrap::Held(Inner::A((a, b)) | Inner::B((b, a)), _) => {
            let n = a.n + b.n;
            drop(a);
            drop(b);
            n
        }
        Wrap::Held(_, rest) => {
            let n = rest.n;
            drop(rest);
            n
        }
        Wrap::Empty => 0,
    }
}

/// A `Token` that can be a map key, so the finisher below has a real receiver.
#[derive(Hash, PartialEq, Eq)]
pub struct Name {
    pub text: String,
}

pub struct Counts {
    pub m: std::collections::HashMap<Name, Result<u32, u32>>,
}

impl Counts {
    /// A call the engine REFUSES: `or_default()` needs the value type's
    /// default, and a `Result` has none the port can write. The KEY is moved
    /// into `entry(k)` in Rust, but the hole throws before the entry is ever
    /// made — so the key is still this block's, and its unwind releases it.
    pub fn finish(&mut self, k: Name) -> u32 {
        self.m.entry(k).or_default();
        0
    }
}
