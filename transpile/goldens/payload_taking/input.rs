//! What one element's pattern takes out of the value that element holds —
//! asked once, per element, through the wrappers a pattern may be written
//! behind. K4, K5, K12, K15, K16.
//!
//! For: a variant's payload member and a tuple's element are the same question,
//! and the port used to answer it in three places with three answers. The
//! `Result` side REFUSED an inner pattern that took a droppable name out of the
//! payload, because the port cannot release an object minus one field; the
//! plain enum arm merely left that member out of its `dropUnbound` list and
//! carried on, so what the pattern did not take leaked with no word said.
//! Neither looked through an `|`. And a tuple's elements were not looked at at
//! all: their borrowedness came from the tuple, which is not a reference even
//! when every element of it is.

pub struct Token {
    pub n: u32,
}

impl Token {
    pub fn new(n: u32) -> Token {
        Token { n }
    }
}

pub enum Inner {
    X(Token),
    Y(Token),
}

pub enum Outer {
    W(Inner),
    Z,
}

/// K4: the arm takes `t` OUT of the `Inner` that the `W` variant holds, and
/// nothing releases the `Inner` that is left behind. The port cannot release an
/// object minus a field, so the arm refuses — and releases the payload the key
/// handed it before it throws. The arm below still runs for a `W` this pattern
/// does not match.
pub fn inside(o: Outer) -> u32 {
    match o {
        Outer::W(Inner::X(t)) => {
            let n = t.n;
            drop(t);
            n
        }
        Outer::W(_) => 1,
        Outer::Z => 0,
    }
}

/// K5: the same question written through an `|`. The scan that answered it saw
/// only the alternation and said the member was untouched, so the `Inner`
/// leaked on every call.
pub fn either(o: Outer) -> u32 {
    match o {
        Outer::W(Inner::X(t) | Inner::Y(t)) => {
            let n = t.n;
            drop(t);
            n
        }
        Outer::Z => 0,
    }
}

pub enum Count {
    Small(u32),
    Large(u32),
}

pub enum Holder {
    Held(Count),
    Empty,
}

/// The other side of the same rule: the pattern reaches inside the member and
/// takes nothing DROPPABLE out, so the member is whole and the arm releases it.
/// Written as a partial move this would refuse a shape the port can write.
pub fn counted(h: Holder) -> u32 {
    match h {
        Holder::Held(Count::Small(n) | Count::Large(n)) => n,
        Holder::Empty => 0,
    }
}

/// K16: a tuple of BORROWED `Result`s. The tuple is not a reference even though
/// both of its elements are, so one borrowedness flag for the whole subject
/// said "owned" and each side called `unwrap()` — taking the wrapper apart and
/// marking it moved, on a `Result` the caller still holds. Every later read of
/// either one was `Result was used after being moved`.
pub fn both(left: &Result<Token, u32>, right: &Result<Token, u32>) -> u32 {
    match (left, right) {
        (Ok(l), Ok(r)) => l.n + r.n,
        _ => 0,
    }
}

/// K15: a consuming match over a TUPLE whose elements own something. A tuple
/// has no path for the payload lookup to answer for, so the scan said the match
/// took nothing out of its subject.
pub fn consumed(pair: (Token, Token)) -> u32 {
    match pair {
        (a, b) => {
            let n = a.n + b.n;
            drop(a);
            drop(b);
            n
        }
    }
}
