//! W1/X1/X2/W2: a lift's release obligation is discharged by a transfer the
//! port actually WROTE, and the flag that reports it stands below everything
//! that can throw.
//!
//! An argument the port lifts above a move flag holds a value Rust had not
//! built yet, so nobody owns it until the call it was lifted for takes it.
//! Two things went wrong with that.
//!
//! Where the call is never written at all — the port refused the terminal of
//! the chain it stood in, so the whole expression came out as one hole — the
//! flag was set anyway, immediately above the hole, and the `finally` believed
//! it: `storage-indexeddb/collection.ts` lifted `order_by_spill.clone()` for a
//! `top_k` the port has no construction for, and released the clone nowhere.
//! Its sibling arm lifted a second clone with no flag at all and no release
//! either.
//!
//! And where the call IS written, the flag stood above the whole of the
//! statement's own text rather than below the operands still to be evaluated:
//! `Event { entity_id: self.id, .. }` reads a field, which cannot panic in
//! Rust, but the port writes `this.deref().id` because the entity is behind a
//! `Deref` — and `deref()` on a value somebody has dropped throws, with the
//! flag already saying the constructor had taken the collection.

use std::ops::Deref;

pub struct Token {
    pub n: u32,
}

#[derive(Clone)]
pub struct Spill {
    pub n: u32,
}

pub struct Rows {
    pub n: u32,
}

impl Rows {
    /// The two-argument adaptor whose terminal refuses: the clone is lifted
    /// for THIS call, and the cast that follows it is why the clone carries a
    /// flag rather than standing last.
    pub fn top_k(self, spill: Spill, k: usize) -> Vec<Token> {
        let _ = (spill, k);
        Vec::new()
    }
}

pub fn tally<T>(x: T) -> u32 {
    let _ = x;
    0
}

/// The lift whose consuming call is a HOLE.
///
/// `collect` into a type the engine cannot name refuses, and with it goes the
/// `top_k` the clone was lifted for. Nothing in the emitted text names the
/// clone, so nothing takes it.
pub fn refused_callee(rows: Rows, spill: Spill, limit: Option<u32>, leave: bool) -> u32 {
    let held = rows;
    if leave {
        return 0;
    }
    match limit {
        Some(k) => tally(
            held.top_k(spill.clone(), k as usize)
                .into_iter()
                .map(|t| t)
                .collect(),
        ),
        None => 0,
    }
}

/// The same, for a lift that stands LAST and so carries no flag: X2's sibling
/// site. The obligation is the lift's, not the flag's.
pub fn refused_callee_unflagged(
    tokens: Vec<Token>,
    spill: Spill,
    limit: Option<u32>,
    leave: bool,
) -> u32 {
    let held = tokens;
    if leave {
        return 0;
    }
    match limit {
        Some(k) => {
            let _ = k;
            held.into_iter().zip(vec![spill.clone()]).collect()
        }
        None => 0,
    }
}

pub struct Inner {
    pub n: u32,
}

/// A `Deref` the port writes as a call that can THROW, which is the whole of
/// W2: `core/node.ts` writes `this.deref().value.id` for `self.id`, and the
/// `value` there reaches an `Arc` that may already have been released. An
/// `unwrap` stands in for it because it throws an ordinary error rather than
/// poisoning the runtime, and what is under test is where the flag stands, not
/// which error the deref raises.
pub struct Handle(pub Option<Inner>);

impl Deref for Handle {
    type Target = Inner;
    fn deref(&self) -> &Inner {
        self.0.as_ref().unwrap()
    }
}

pub struct Event {
    pub token: Token,
    pub n: u32,
}

impl Handle {
    /// The throwing RECEIVER: `self.n` is a field read in Rust and
    /// `this.deref().n` in the port, so it is lifted above the flag with
    /// every other evaluand.
    pub fn make(&self, token: Token, leave: bool) -> u32 {
        if leave {
            return 0;
        }
        let e = Event { token, n: self.n };
        e.n
    }
}
