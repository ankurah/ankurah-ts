// `Ord::cmp` and `PartialOrd::partial_cmp` are both `compareTo` here, and R9
// settled the case where the partial one FORWARDS to the total one: they are the
// same method, so the forwarding body is not written and the one with something
// in it keeps the name. This is the other case. A partial order that is written
// OUT is a body of its own — Rust lets it disagree with the total one, and
// refuse to answer at all — and one name cannot hold both. Whichever the source
// wrote first took `compareTo`: `Weight(0)` compared as though it were
// comparable, because `Ord::cmp` ran for every call.

use std::cmp::Ordering;

pub struct Weight(pub u32);

impl PartialEq for Weight {
    fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
}
impl Eq for Weight {}

impl Ord for Weight {
    fn cmp(&self, other: &Self) -> Ordering { self.0.cmp(&other.0) }
}

/// A REAL partial order: it refuses to compare a zero with anything.
impl PartialOrd for Weight {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.0 == 0 || other.0 == 0 {
            return None;
        }
        Some(self.0.cmp(&other.0))
    }
}

pub struct Plain(pub u32);

impl PartialEq for Plain {
    fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
}
impl Eq for Plain {}
impl Ord for Plain {
    fn cmp(&self, other: &Self) -> Ordering { self.0.cmp(&other.0) }
}
/// The forwarding one, which is the same method here.
impl PartialOrd for Plain {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
