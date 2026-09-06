//! What a consuming arm OWES the payload it was handed.
//!
//! For: `intoMatch` hands the payload to the arm and releases nothing of its
//! own on any path out, so from the moment the key is called the whole payload
//! is the arm's — the parts its pattern named, the parts it wrote `_` for, and
//! the wrapper a nested pattern reached inside without taking anything out of.
//! Every path out of the arm settles all of them: the body's own `finally`, the
//! guard's `catch` where the guard throws before the body is entered, and the
//! release a hole writes before it stops.

use crate::body::BodyTranslator;

/// A HOLE written where an arm holds the payload, with what it owes first.
///
/// R12 says a hole throws where the branch would have run. It does not say the
/// branch may abandon what it was handed: `intoMatch` marks the subject moved
/// and gives the payload to the arm, and releases nothing of its own on any path
/// out — so an arm that throws still owns the whole payload, and a refusal that
/// walked away from it turned a reported gap into a leak. The release stands
/// BEFORE the throw rather than in a `finally`, because there is no other path
/// out of a block whose only statement throws.
pub(super) fn hole_in_an_arm(
    what: &str,
    param: &str,
    has_payload: bool,
    takes: crate::ownership::scrutinee::Takes,
) -> String {
    let throw = format!("{};\n", crate::body::hole_text(what));
    if !has_payload || takes != crate::ownership::scrutinee::Takes::Payload {
        return throw;
    }
    format!("dropUnbound({}, []);\n{}", param, throw)
}

/// What an arm owes when its own DECLARATIONS carry a hole.
///
/// A refusal written into the bindings — `const path = unsupported(..)`, which
/// is what an or-pattern the translator cannot read back comes out as — throws
/// before the `try` around the body is entered, so the `finally` that would
/// have released the rest of the payload never runs. The release goes first,
/// for the same reason `hole_in_an_arm` puts it first: `intoMatch` releases
/// nothing of its own, and the arm is the owner from the moment it is called.
pub(super) fn release_before_a_hole_in_the_bindings(
    bindings: &str,
    param: &str,
    has_payload: bool,
    takes: crate::ownership::scrutinee::Takes,
) -> String {
    if !has_payload
        || takes != crate::ownership::scrutinee::Takes::Payload
        || !bindings.contains("unsupported(")
    {
        return String::new();
    }
    format!("dropUnbound({}, []);\n", param)
}


/// Does this pattern leave part of the variant's payload with no name?
///
/// Rust makes a tuple or struct pattern name every slot unless it writes `..`,
/// so the only unnamed parts are the ones the source wrote `_` for and the ones
/// a `..` stands in for. A consuming arm owes those a release, because nothing
/// else holds them any more.
pub(super) fn leaves_payload_unbound(pat: &syn::Pat, t: &BodyTranslator) -> bool {
    let unowned = |p: &syn::Pat| {
        matches!(p, syn::Pat::Rest(_)) || super::taking::element_is_left_whole(p, t)
    };
    match pat {
        syn::Pat::TupleStruct(ts) => ts.elems.iter().any(unowned),
        syn::Pat::Struct(st) => st.rest.is_some() || st.fields.iter().any(|f| unowned(&f.pat)),
        // `E::Unit` names a variant with no payload: Rust rejects the path form
        // for a variant that carries one.
        _ => false,
    }
}

/// What a link owes if its GUARD throws.
///
/// `intoMatch` hands the payload to the arm and releases nothing of its own on
/// any path, so from the moment the key is called the whole payload is the
/// arm's. The arm's `finally` covers the body; a guard runs before that
/// `finally` is entered, and a guard that panicked left the names the pattern
/// bound, and the payload members nothing bound, to the collector. Nothing has
/// moved yet at that point, so each release is unconditional.
pub(super) fn guard_release(
    declared: &[String],
    release_rest: &str,
    takes: crate::ownership::scrutinee::Takes,
    t: &BodyTranslator,
) -> String {
    if takes != crate::ownership::scrutinee::Takes::Payload {
        return String::new();
    }
    // Asked with an empty body: no statement of the arm has run when the guard
    // is being made, so no name is moved and every droppable one is owed.
    let owed = t.claim_bindings(declared, &[]);
    let mut out = String::new();
    for value in owed.iter().rev() {
        out.push_str(&value.release());
    }
    out.push_str(release_rest);
    out
}

/// What a consuming arm's outermost `finally` says about the parts of the
/// payload no name took.
pub(super) fn release_of(
    case: &syn::Pat,
    param: &str,
    bound: &[String],
    takes: crate::ownership::scrutinee::Takes,
    t: &BodyTranslator,
) -> String {
    match takes {
        crate::ownership::scrutinee::Takes::Payload if leaves_payload_unbound(case, t) =>
        {
            format!(
                "dropUnbound({}, [{}]);\n",
                param,
                bound.iter().map(|k| format!("'{}'", k)).collect::<Vec<_>>().join(", ")
            )
        }
        _ => String::new(),
    }
}
