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

use crate::ownership::arm_takes::{binds_by_value, members_of};
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
    patterns: &[&syn::Pat],
    payload_of: impl Fn(&syn::Path) -> Vec<(String, Ty)>,
) -> Takes {
    // A `&T` subject cannot be moved out of, whatever the arms say.
    if matches!(subject, Ty::Ref { .. }) {
        return Takes::Nothing;
    }
    if !drops_of(probe, subject).is_droppable() {
        return Takes::Nothing;
    }
    let moved = patterns
        .iter()
        .any(|pat| binds_owned_payload(probe, pat, subject, &payload_of));
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
    subject: &Ty,
    payload_of: &impl Fn(&syn::Path) -> Vec<(String, Ty)>,
) -> bool {
    match pat {
        // A member is paired with its DECLARED type by `members_of`: by
        // position for a tuple-struct pattern and BY NAME for a struct one.
        // Zipped positionally, `Named::V { held, .. }` over
        // `V { copy: u32, held: Token }` paired `held` with the `u32`, said the
        // arm took nothing droppable, and the borrowing form left the token
        // owned by the binding AND by the enum's cascade (I2).
        syn::Pat::TupleStruct(_) | syn::Pat::Struct(_) => {
            let path = match pat {
                syn::Pat::TupleStruct(ts) => &ts.path,
                syn::Pat::Struct(s) => &s.path,
                _ => unreachable!("the arm above matched one of these two"),
            };
            let fields = payload_of(path);
            members_of(pat, &fields).into_iter().any(|(sub, ty)| match ty {
                Some(ty) => binds_by_value(sub) && drops_of(probe, ty).is_droppable(),
                // A member the payload could not name is not counted, which is
                // where this has always stood: `payload_of` is asked with the
                // SUBJECT's own type, so a pattern nested inside an element —
                // `(RangeBound::Included(l), ..)` over a tuple — comes back
                // with nothing at all. Reading that as "takes something" made
                // `core/collation.rs`'s `is_in_range` consuming and released
                // two bounds the caller still owned. Answering "takes nothing"
                // leaves the subject's own release standing, which is what the
                // block already writes.
                None => false,
            })
        }
        // K15: `match (a, b) { (Some(x), _) => .. }` takes `x` out of the
        // tuple's FIRST element, and a tuple has no path for `payload_of` to
        // answer for — so this used to say the match took nothing, the value
        // lowering claimed no binding, and a binding the arm then moved was
        // released a second time by the tuple's own release.
        syn::Pat::Tuple(tuple) => match subject {
            Ty::Tuple(elements) => tuple
                .elems
                .iter()
                .zip(elements)
                .any(|(sub, ty)| element_binds_owned(probe, sub, ty, payload_of)),
            // A tuple pattern against something the engine could not read as a
            // tuple: any by-value binding in it may own something.
            _ => tuple.elems.iter().any(binds_by_value),
        },
        syn::Pat::Or(or) => or
            .cases
            .iter()
            .any(|case| binds_owned_payload(probe, case, subject, payload_of)),
        syn::Pat::Paren(p) => binds_owned_payload(probe, &p.pat, subject, payload_of),
        // `x => ..` binds the whole subject, not a part of it: the arm owns it
        // from there, which the binding's own scope handles.
        _ => false,
    }
}

/// One tuple ELEMENT: does its pattern take something the arm has to release
/// out of the value that element holds?
///
/// A name for the whole element owns it when the element is droppable; a
/// pattern that reaches inside asks the same question of what it finds there.
fn element_binds_owned(
    probe: &Probe,
    pat: &syn::Pat,
    ty: &Ty,
    payload_of: &impl Fn(&syn::Path) -> Vec<(String, Ty)>,
) -> bool {
    match pat {
        syn::Pat::TupleStruct(_) | syn::Pat::Struct(_) | syn::Pat::Tuple(_) | syn::Pat::Or(_) => {
            binds_owned_payload(probe, pat, ty, payload_of)
        }
        _ => binds_by_value(pat) && drops_of(probe, ty).is_droppable(),
    }
}

/// Does this arm's pattern take the whole subject into a name of its own?
///
/// `other => ..` binds the subject itself rather than a part of it, so Rust
/// moves the subject into that binding on the path where that arm runs — and on
/// no other path, which is what a drop flag is for. A `_` takes nothing and
/// leaves the subject to be dropped where the match ends.
pub fn binds_whole_subject(pat: &syn::Pat) -> bool {
    match pat {
        syn::Pat::Ident(ident) => {
            ident.by_ref.is_none() && ident.subpat.is_none() && ident.ident != "_"
        }
        syn::Pat::Paren(p) => binds_whole_subject(&p.pat),
        _ => false,
    }
}
