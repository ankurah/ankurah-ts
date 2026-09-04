//! An arm that names no variant — `_` or a plain name — runs for everything the
//! arms above it did not name. The runtime's `match` dispatches on the variant
//! name, so such an arm has no key to be written under: the variants the other
//! arms name are tested first, and the catch-all takes what is left with the
//! subject still whole.

pub struct Inner {
    pub width: usize,
}

pub enum Order {
    Less,
    Equal,
    Greater,
}

pub enum Cause {
    Denied(Inner),
    Missing,
    Other,
}

pub enum Wrapped {
    Held(Inner),
    Whole(Cause),
}

/// A named catch-all hands back the value it was given. Every non-tie
/// comparison in a sort takes this arm, which is why it dropping made the sort
/// fatal on its first unequal pair.
pub fn tie_break(order: Order, fallback: Order) -> Order {
    match order {
        Order::Equal => fallback,
        other => other,
    }
}

/// A consuming match: the `Denied` arm takes the payload out, and the `_` arm
/// reads the subject the test has left whole — which is what Rust means by a
/// `_` arm moving nothing.
pub fn lift(cause: Cause) -> Wrapped {
    match cause {
        Cause::Denied(inner) => Wrapped::Held(inner),
        _ => Wrapped::Whole(cause),
    }
}

/// Several named arms make one test between them.
pub fn rank(cause: &Cause) -> usize {
    match cause {
        Cause::Denied(inner) => inner.width,
        Cause::Missing => 1,
        _ => 0,
    }
}

/// A catch-all in statement position runs for its effect alone.
pub fn widen(cause: &Cause, into: &mut Vec<usize>) {
    match cause {
        Cause::Denied(inner) => into.push(inner.width),
        _ => into.push(0),
    }
}

pub fn count(cause: &Cause) -> usize {
    match cause {
        Cause::Denied(inner) => inner.width,
        _ => 1,
    }
}

/// A named catch-all that only reads what it was given releases it itself,
/// exactly as Rust drops that binding at the end of the arm.
pub fn tally(cause: Cause) -> usize {
    match cause {
        Cause::Denied(inner) => inner.width,
        rest => count(&rest),
    }
}
