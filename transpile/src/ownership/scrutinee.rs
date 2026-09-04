//! Does a `match` take its subject apart, or read it and leave it whole?
//!
//! Rust moves out of a `match` when an arm binds part of the subject by value,
//! and reads through it otherwise. The two need different code: `intoMatch`
//! hands the payload to the arm and leaves the enum moved, while `match` lends
//! it and leaves the enum for its owner to drop. Emitting the borrowing form
//! where Rust moved left the payload owned twice — once by the arm that got it
//! and once by the cascade of the enum nobody had marked moved.
//!
//! The question is answered from the subject's resolved type and the arms'
//! patterns, because that is what Rust answers it from.

use crate::ownership::drops_of;
use crate::registry::Probe;
use crate::ty::Ty;

/// What a `match` does to its subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Takes {
    /// An arm binds a payload that owes a release, so the subject's contents
    /// are handed over: `intoMatch`, and no drop after it.
    Payload,
    /// Every arm reads through the subject: `match`, and its owner still drops
    /// it.
    Nothing,
}

/// Whether this match hands its subject's payload to an arm.
///
/// `payload_of` answers "what does the variant this pattern names hold", which
/// only the type context can say; the rest is the pattern.
pub fn takes(
    probe: &Probe,
    subject: &Ty,
    arms: &[syn::Arm],
    payload_of: impl Fn(&syn::Path) -> Vec<Ty>,
) -> Takes {
    // A `&T` subject cannot be moved out of, whatever the arms say.
    if matches!(subject, Ty::Ref { .. }) {
        return Takes::Nothing;
    }
    if !drops_of(probe, subject).is_droppable() {
        return Takes::Nothing;
    }
    let moved = arms
        .iter()
        .any(|arm| binds_owned_payload(probe, &arm.pat, &payload_of));
    if moved {
        Takes::Payload
    } else {
        Takes::Nothing
    }
}

/// Does this pattern bind, by value, something the arm then has to release?
fn binds_owned_payload(
    probe: &Probe,
    pat: &syn::Pat,
    payload_of: &impl Fn(&syn::Path) -> Vec<Ty>,
) -> bool {
    match pat {
        syn::Pat::TupleStruct(ts) => {
            let fields = payload_of(&ts.path);
            ts.elems
                .iter()
                .zip(fields)
                .any(|(sub, ty)| binds_by_value(sub) && drops_of(probe, &ty).is_droppable())
        }
        syn::Pat::Struct(s) => {
            let fields = payload_of(&s.path);
            // A named-field variant is matched by name, so the arm's order says
            // nothing; the payload list is in declaration order and the pattern
            // reads as many of them as it names.
            s.fields
                .iter()
                .zip(fields)
                .any(|(f, ty)| binds_by_value(&f.pat) && drops_of(probe, &ty).is_droppable())
        }
        syn::Pat::Or(or) => or
            .cases
            .iter()
            .any(|case| binds_owned_payload(probe, case, payload_of)),
        syn::Pat::Paren(p) => binds_owned_payload(probe, &p.pat, payload_of),
        // `x => ..` binds the whole subject, not a part of it: the arm owns it
        // from there, which the binding's own scope handles.
        _ => false,
    }
}

/// A name bound with neither `ref` nor `&`, which is Rust's by-value binding.
fn binds_by_value(pat: &syn::Pat) -> bool {
    match pat {
        syn::Pat::Ident(ident) => ident.by_ref.is_none(),
        syn::Pat::Paren(p) => binds_by_value(&p.pat),
        _ => false,
    }
}
