//! What a `for` loop owns, and when it lets go of it.
//!
//! `for entity in entities` hands the loop the whole sequence: Rust's
//! `IntoIterator` takes it by value, each turn takes one element out, and the
//! iterator drops whatever is left when the loop stops — which is what a
//! `break` or a `return` out of the body does. Emitting a plain `for … of`
//! released none of that: the element each turn bound was never dropped, and an
//! early exit abandoned the rest of the sequence.
//!
//! A loop over `&entities` owns nothing and needs none of this; the item type
//! says which of the two this is.

use crate::registry::Probe;
use crate::ty::Ty;

/// How a `for` loop over this sequence has to be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Iterate {
    /// `for … of`: the loop borrows, and the elements belong to somebody else.
    Borrowed,
    /// The sequence is a JavaScript array the loop owns, so it is walked by
    /// index and the tail it never reached is released at the end.
    OwnedArray,
    /// The loop owns the sequence, but the runtime does not write it as an
    /// array — a `HashMap`, an iterator adapter — so there is no way to name
    /// the elements the loop did not reach. Reported.
    OwnedOpaque,
}

/// Which form this loop needs, from the sequence's type and the item's.
///
/// The item decides whether anything is owned at all: `IntoIterator for
/// Vec<T>` hands out a `T` and `IntoIterator for &Vec<T>` hands out a `&T`,
/// and the second owns nothing.
pub fn iterate(probe: &Probe, sequence: Option<&Ty>, item: Option<&Ty>) -> Iterate {
    let owns_items = item.is_some_and(|item| crate::ownership::drops_of(probe, item).is_droppable());
    if !owns_items {
        return Iterate::Borrowed;
    }
    match sequence {
        Some(ty) if is_array(probe, ty) => Iterate::OwnedArray,
        _ => Iterate::OwnedOpaque,
    }
}

/// Is this a sequence the runtime writes as a JavaScript array?
fn is_array(probe: &Probe, ty: &Ty) -> bool {
    match ty {
        Ty::Array { .. } | Ty::Slice(_) => true,
        Ty::Ref { .. } => false,
        Ty::Named { id, .. } => probe
            .reg
            .system_type("std::vec::Vec")
            .is_some_and(|vec| vec == *id),
        _ => false,
    }
}

/// The owned-array loop: one element out per turn, and the tail released when
/// the loop stops for any reason.
///
/// `at` is the index the next turn would read, so the `finally` releases
/// exactly what `next()` never handed out — which is what dropping Rust's
/// `IntoIter` does.
pub fn owned_array_loop(sequence: &str, at: &str, binding: &str, body: &str) -> String {
    format!(
        "let {at} = 0;\n\
         try {{\n  \
           while ({at} < {sequence}.length) {{\n    \
             const {binding} = {sequence}[{at}++];\n\
{body}  \
           }}\n\
         }} finally {{\n  \
           dropOwned({sequence}.slice({at}));\n\
         }}",
        at = at,
        sequence = sequence,
        binding = binding,
        body = crate::body::indent(&crate::body::indent(body)),
    )
}
