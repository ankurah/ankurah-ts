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

/// The value position: a match that is not the function's own return still has
/// to hand its value back. The expanded arms used to be written by hand rather
/// than through the arm renderer, and in this position they produced a body
/// with no `return` at all, so the local was `undefined` for every value the
/// named arms did not cover.
pub fn let_init(cause: &Cause) -> usize {
    let picked = match cause {
        Cause::Denied(inner) => inner.width,
        _ => 2,
    };
    picked + 1
}

/// The same, as a call argument.
pub fn as_argument(cause: &Cause) -> usize {
    count_twice(match cause {
        Cause::Denied(inner) => inner.width,
        _ => 3,
    })
}

fn count_twice(n: usize) -> usize {
    n * 2
}

/// Every variant carries something, so a consuming `_` arm has a payload to
/// own however little of it the arm reads.
pub enum Held {
    First(Inner),
    Second(Inner),
    Third(Inner),
}

/// A consuming `_` arm that reads nothing still owns the payload: `intoMatch`
/// hands the whole thing over and keeps none of it, so an arm that takes no
/// name for it is what leaks it.
pub fn ignore(held: Held) -> usize {
    match held {
        Held::First(inner) => inner.width,
        _ => 0,
    }
}

/// A named arm that ignores its own payload owns it just the same.
pub fn ignore_named(held: Held) -> usize {
    match held {
        Held::First(_) => 1,
        Held::Second(inner) => inner.width,
        Held::Third(_) => 3,
    }
}

/// An arm that tests INSIDE its variant does not cover the variant: the values
/// it does not match still belong to the catch-all. Counting it by the variant
/// name alone deleted the catch-all and left those values with no arm at all.
pub enum Reason {
    Cause(Cause),
    Plain,
}

pub fn refutable(reason: &Reason) -> usize {
    match reason {
        Reason::Cause(Cause::Missing) => 5,
        _ => 6,
    }
}

/// A catch-all that binds the scrutinee's own name binds it once. This used to
/// write `const cause` twice — the binding and the reconstruction — which no
/// JavaScript engine will load.
pub fn same_name(cause: Cause) -> Cause {
    match cause {
        Cause::Denied(inner) => Cause::Denied(inner),
        cause => cause,
    }
}

/// An unwind out of a consuming arm has ONE owner: the arm. Its `finally`
/// releases what it bound, and `intoMatch` releases nothing of its own — two
/// owners made the panic come back as `BUG: Inner was dropped twice`.
pub fn unwind(cause: Cause) -> usize {
    match cause {
        Cause::Denied(inner) => panic!("width {} is not allowed", inner.width),
        _ => 0,
    }
}
