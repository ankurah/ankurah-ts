//! ONE answer to "what does this pattern take out of the value it is written
//! for", for both questions the port asks about a `match`.
//!
//! For: the port asks that question twice. `ownership/scrutinee.rs` asks it of
//! the WHOLE subject — does any arm move part of it, so that the match is
//! written as `intoMatch` and the subject's owner stops releasing it — and
//! `match_expr/taking.rs` asks it of ONE member — does this member still have
//! an owner after the pattern has run. The two answered from different code and
//! shared no identity resolution, no member mapping and no disposition for a
//! partially named tuple, and three ownership defects came straight out of the
//! gap:
//!
//! * a struct-variant pattern was zipped with the payload POSITIONALLY, so
//!   `Named::V { held, .. }` over `V { copy: u32, held: Token }` paired `held`
//!   with `copy: u32`, decided the arm took nothing droppable, wrote the
//!   borrowing form — and `held.drop()` was followed by the enum's own cascade
//!   into the same token: `BUG: Token was dropped twice`;
//! * a tuple pattern that named some of its elements claimed the subject and
//!   released only what it named, so `(a, _)` over two droppable elements left
//!   the second with no owner at all;
//! * `Some` was decided by SPELLING, so a user enum's `Maybe::Some(t)` was read
//!   as the nullable `Option` and the `Maybe` around `t` was never released.
//!
//! So the shape decisions live here, once, and both callers act on them:
//! members by NAME, elements by INDEX, `Some` by identity, and every one of
//! them through `|`, parentheses and `&`.

use crate::ownership::drops_of;
use crate::registry::Probe;
use crate::ty::Ty;

/// A name bound with neither `ref` nor `&`, which is Rust's by-value binding.
///
/// A pattern that goes further into the payload before it binds still moves out
/// of it: `Ex::Literal(Lit::I(i))` takes `i` by value out of the `Lit` the
/// `Literal` variant holds, so Rust moves the whole subject just as
/// `Ex::Path(i)` does. Reading only the outermost pattern said this match
/// borrowed, so the arm took the value AND the subject's owner released it —
/// the same double drop the borrowing form was written to avoid.
pub fn binds_by_value(pat: &syn::Pat) -> bool {
    match pat {
        syn::Pat::Ident(ident) => ident.by_ref.is_none(),
        syn::Pat::Paren(p) => binds_by_value(&p.pat),
        syn::Pat::TupleStruct(ts) => ts.elems.iter().any(binds_by_value),
        syn::Pat::Struct(st) => st.fields.iter().any(|f| binds_by_value(&f.pat)),
        syn::Pat::Tuple(tuple) => tuple.elems.iter().any(binds_by_value),
        syn::Pat::Or(or) => or.cases.iter().any(binds_by_value),
        // `&x` matches through a reference and binds one.
        syn::Pat::Reference(_) => false,
        _ => false,
    }
}

/// The sub-pattern standing for each member of a payload, with that member's
/// DECLARED type.
///
/// A tuple-struct pattern names its members by POSITION and a struct pattern
/// names them BY NAME — `Named::V { held, .. }` says nothing about where `held`
/// is declared, and zipping the two lists paired it with the field declared
/// first. A member the pattern does not mention is not in the answer at all;
/// a member it mentions whose type the payload does not carry comes back with
/// `None`, which every caller reads as "the engine cannot say".
pub fn members_of<'p, 't>(
    pat: &'p syn::Pat,
    payload: &'t [(String, Ty)],
) -> Vec<(&'p syn::Pat, Option<&'t Ty>)> {
    match pat {
        syn::Pat::Paren(p) => members_of(&p.pat, payload),
        syn::Pat::Reference(r) => members_of(&r.pat, payload),
        // `E::V(a, b)`: position by position, and a `..` stands for the members
        // it skipped, which this answer simply does not mention.
        syn::Pat::TupleStruct(ts) => ts
            .elems
            .iter()
            .take_while(|p| !matches!(p, syn::Pat::Rest(_)))
            .enumerate()
            .map(|(at, sub)| (sub, payload.get(at).map(|(_, ty)| ty)))
            .collect(),
        // `E::V { held, .. }`: by name. The payload spells a member the way the
        // EMISSION does — `index_spec` is `indexSpec` there — so the pattern's
        // Rust ident is put through the same map before it is looked up; a
        // snake_case field found nothing at all otherwise, and the answer came
        // back "this member has no type" for every field of more than one word.
        // `Member::Unnamed` is how a tuple variant is written in struct form
        // (`E::V { 0: x }`), and the payload spells that member `_0`.
        syn::Pat::Struct(s) => s
            .fields
            .iter()
            .map(|f| {
                let name = match &f.member {
                    syn::Member::Named(ident) => {
                        crate::name_map::to_camel_case(&ident.to_string())
                    }
                    syn::Member::Unnamed(index) => format!("_{}", index.index),
                };
                let ty = payload.iter().find(|(n, _)| *n == name).map(|(_, ty)| ty);
                (&*f.pat, ty)
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Which positions of a tuple or slice pattern leave the element they stand
/// for with no owner, or `None` where the port cannot count them.
///
/// `(a, _)` over two elements moves the first into `a` and leaves the second
/// where it is; Rust drops it when the match ends. The port writes a tuple as an
/// array and knows every position of it, so the caller can release those
/// positions by index. `None` is the shape this cannot count: a `..`, whose
/// positions are counted from the END, and a pattern naming a different number
/// of positions than the value has.
pub fn unowned_positions(pat: &syn::Pat, len: usize) -> Option<Vec<usize>> {
    if has_a_rest(pat) {
        return None;
    }
    let elements: Vec<&syn::Pat> = element_patterns(pat)?;
    if elements.len() != len {
        return None;
    }
    Some(
        elements
            .iter()
            .enumerate()
            .filter(|(_, sub)| !binds_by_value(sub))
            .map(|(at, _)| at)
            .collect(),
    )
}

/// The positions of a tuple or slice pattern that a droppable element sits at
/// and no name owns — what a consuming arm has to release itself.
pub fn unowned_droppable_positions(
    pat: &syn::Pat,
    elements: &[Ty],
    probe: &Probe,
) -> Option<Vec<usize>> {
    let unowned = unowned_positions(pat, elements.len())?;
    Some(
        unowned
            .into_iter()
            .filter(|at| drops_of(probe, &elements[*at]).is_droppable())
            .collect(),
    )
}

/// The sub-patterns of a tuple or slice pattern, through parentheses and `&`.
fn element_patterns(pat: &syn::Pat) -> Option<Vec<&syn::Pat>> {
    match pat {
        syn::Pat::Paren(p) => element_patterns(&p.pat),
        syn::Pat::Reference(r) => element_patterns(&r.pat),
        syn::Pat::Tuple(tuple) => Some(tuple.elems.iter().collect()),
        syn::Pat::Slice(slice) => Some(slice.elems.iter().collect()),
        _ => None,
    }
}

fn has_a_rest(pat: &syn::Pat) -> bool {
    match element_patterns(pat) {
        Some(elems) => elems.iter().any(|p| matches!(p, syn::Pat::Rest(_))),
        None => false,
    }
}

/// `Some(x)` with `x` a plain name, and `Some` the PRELUDE's: the one shape
/// whose test leaves no wrapper, because the port writes `Option<T>` as
/// `T | null` and `x` IS the value.
///
/// `is_option` answers whether the path names `std::option::Option`'s variant,
/// which is the only thing that makes the wrapper vanish. Decided by spelling,
/// a user enum's `Maybe::Some(t)` was read the same way and the `Maybe` around
/// `t` was released by nobody.
pub fn takes_the_whole_nullable(pat: &syn::Pat, is_option: &dyn Fn(&syn::Path) -> bool) -> bool {
    let syn::Pat::TupleStruct(ts) = pat else { return false };
    let Some(leaf) = ts.path.segments.last() else { return false };
    if leaf.ident != "Some" || ts.elems.len() != 1 || !is_option(&ts.path) {
        return false;
    }
    matches!(ts.elems.first(), Some(syn::Pat::Ident(ident)) if ident.subpat.is_none())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ty::Ty;

    fn pat(text: &str) -> syn::Pat {
        // `Pat` has no `Parse` of its own in syn 2: a pattern is ambiguous
        // between one alternative and several, and the caller says which.
        syn::parse::Parser::parse_str(syn::Pat::parse_multi_with_leading_vert, text)
            .unwrap_or_else(|e| panic!("cannot parse `{text}`: {e}"))
    }

    /// A stand-in type: these tests are about the SHAPE decisions, which
    /// need no registry. What is droppable is the probe's answer and is
    /// tested where a probe exists — `goldens/arm_takes`.
    fn named(name: &str) -> Ty {
        Ty::Param(name.to_string())
    }

    /// I2: a struct pattern names its members, and the order it names them in
    /// says nothing about where they are declared.
    #[test]
    fn a_struct_pattern_pairs_its_fields_by_name() {
        let payload = vec![("copy".to_string(), named("u32")), ("held".to_string(), named("Token"))];
        let p = pat("Named::V { held, .. }");
        let found = members_of(&p, &payload);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, Some(&named("Token")), "`held` is the field named `held`");
    }

    /// The payload spells a member the way the EMISSION does, so a pattern
    /// naming a snake_case field has to be put through the same map: looked up
    /// raw, `index_spec` found nothing and every multi-word field came back
    /// unanswered.
    #[test]
    fn a_snake_case_field_is_looked_up_the_way_the_payload_spells_it() {
        let payload = vec![
            ("indexSpec".to_string(), named("KeySpec")),
            ("orderBySpill".to_string(), named("OrderByComponents")),
        ];
        assert_eq!(
            members_of(&pat("Plan::Index { index_spec, .. }"), &payload)[0].1,
            Some(&named("KeySpec"))
        );
        assert_eq!(
            members_of(&pat("Plan::Index { order_by_spill, .. }"), &payload)[0].1,
            Some(&named("OrderByComponents"))
        );
    }

    /// The same shape written the other way round: naming both fields out of
    /// declaration order still pairs each with its own type.
    #[test]
    fn both_fields_named_out_of_order_still_pair_by_name() {
        let payload = vec![("copy".to_string(), named("u32")), ("held".to_string(), named("Token"))];
        let p = pat("Named::V { held, copy }");
        let found = members_of(&p, &payload);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].1, Some(&named("Token")));
        assert_eq!(found[1].1, Some(&named("u32")));
    }

    /// A tuple-struct pattern is the other rule: by position.
    #[test]
    fn a_tuple_struct_pattern_pairs_its_members_by_position() {
        let payload = vec![("_0".to_string(), named("Token")), ("_1".to_string(), named("u32"))];
        let p = pat("E::V(t, n)");
        let found = members_of(&p, &payload);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].1, Some(&named("Token")));
        assert_eq!(found[1].1, Some(&named("u32")));
    }

    /// A member the pattern does not mention is not in the answer, and one the
    /// payload does not carry comes back unanswered.
    #[test]
    fn a_member_the_payload_does_not_carry_is_unanswered() {
        let payload = vec![("copy".to_string(), named("u32"))];
        let p = pat("Named::V { held }");
        assert_eq!(members_of(&p, &payload)[0].1, None);
    }

    /// H2/I1: `(a, _)` leaves the second element with no owner, and the port
    /// knows which position that is.
    #[test]
    fn a_partial_tuple_names_the_positions_it_left_behind() {
        assert_eq!(unowned_positions(&pat("(a, _)"), 2), Some(vec![1]));
        assert_eq!(unowned_positions(&pat("(_, b)"), 2), Some(vec![0]));
        assert_eq!(unowned_positions(&pat("(a, b)"), 2), Some(vec![]));
        assert_eq!(unowned_positions(&pat("(_, _)"), 2), Some(vec![0, 1]));
        // A `ref` binding borrows, so the element it names still has no owner
        // of its own.
        assert_eq!(unowned_positions(&pat("(ref a, b)"), 2), Some(vec![0]));
        // A slice pattern is counted the same way.
        assert_eq!(unowned_positions(&pat("[a, _, c]"), 3), Some(vec![1]));
    }

    /// A `..`, and a pattern of a different length, are the shapes this cannot
    /// count — so it says so and the caller refuses instead of guessing.
    #[test]
    fn a_rest_is_not_counted_by_position() {
        assert_eq!(unowned_positions(&pat("(a, ..)"), 3), None);
        assert_eq!(unowned_positions(&pat("(a, b)"), 3), None);
        assert_eq!(unowned_positions(&pat("Named::V { held }"), 2), None);
    }

    /// I3: the nullable shape is the prelude's `Some`, not everything spelled
    /// that way.
    #[test]
    fn some_is_the_nullable_only_when_it_is_the_preludes() {
        let always = |_: &syn::Path| true;
        let never = |_: &syn::Path| false;
        assert!(takes_the_whole_nullable(&pat("Some(x)"), &always));
        assert!(!takes_the_whole_nullable(&pat("Maybe::Some(t)"), &never));
        // Still not the nullable when the payload is a pattern rather than a
        // name, whoever declares it.
        assert!(!takes_the_whole_nullable(&pat("Some(Inner::X(n))"), &always));
    }

    /// Every shape is answered through `|`, parentheses and `&`.
    #[test]
    fn a_by_value_binding_is_seen_through_the_wrappers() {
        assert!(binds_by_value(&pat("(x)")));
        assert!(binds_by_value(&pat("E::A(x) | E::B(x)")));
        assert!(!binds_by_value(&pat("&x")));
        assert!(!binds_by_value(&pat("ref x")));
    }
}
