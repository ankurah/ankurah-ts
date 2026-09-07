//! Which writer a `match` goes to: the runtime's keyed dispatch, the
//! `Result`'s two reads, or the if-chain.
//!
//! For: the three writers emit different programs — `intoMatch` hands the
//! payload over and marks the subject moved, `isOk()`/`unwrap()` opens the
//! runtime's own wrapper, and the if-chain reads the subject and takes nothing
//! — so choosing the wrong one is not a matter of style. It emitted a program
//! that leaked (an enum nobody released), one that threw (`unwrap` on a class
//! that has none), or one that called a method the value has not got.
//!
//! Both questions are asked of the ALTERNATIVES a pattern offers, not of the
//! pattern as written, because `A(x) | B(x)` is one pattern and two variants;
//! and the `Result` question is asked of the subject's IDENTITY, because a
//! crate may spell its own variant `Ok`.

use super::arms::cases_of;
use crate::body::BodyTranslator;

/// Does any alternative of any arm name a variant of one of the port's enums?
///
/// Read through `cases_of`, so an or-pattern is its alternatives: `match s {
/// Shape::One(t) | Shape::Two(t, _) => .., _ => 0 }` is a `syn::Pat::Or` and a
/// `syn::Pat::Wild`, neither of which names a variant on its own, so the whole
/// match went to the if-chain — which reads `s.value` in place, marks nothing
/// moved and releases neither `s` nor the payload the pattern did not name.
pub(super) fn looks_like_enum_match(m: &syn::ExprMatch, t: &BodyTranslator) -> bool {
    let own = subject_declares_its_own_variants(&m.expr, t);
    m.arms.iter().flat_map(|arm| cases_of(&arm.pat)).any(|pat| names_a_variant(pat, own))
}

/// One alternative: does it name a variant the runtime's keyed match can
/// dispatch on?
///
/// `Some`, `None`, `Ok` and `Err` are the prelude's, and the port writes those
/// two types as a nullable and as its own wrapper — unless the SUBJECT is an
/// enum the corpus declared, in which case they are that enum's variants and
/// nothing else. `enum Outcome { Ok(Token), Other }` was excluded by its leaf,
/// so `if let Outcome::Ok(t) = o` went to the if-chain: `o.value` read in
/// place, `o` marked by nothing and released by nobody.
fn names_a_variant(pat: &syn::Pat, own_enum: bool) -> bool {
    match pat {
        syn::Pat::TupleStruct(ts) => {
            let name = ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
            let prelude = name == "Some" || name == "None" || name == "Ok" || name == "Err";
            (own_enum || !prelude)
                && name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        }
        syn::Pat::Struct(s) => {
            let name = s.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
            name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        }
        syn::Pat::Path(p) => p.path.segments.len() >= 2,
        _ => false,
    }
}

/// Do the arms open the runtime's `Result`?
///
/// The arms have to SPELL `Ok` or `Err` — that is how Rust writes the two
/// variants — and the subject has not to be an enum of the corpus's own. A
/// crate that declares `enum Outcome { Ok(Token), Other }` writes
/// `Outcome::Ok(token)` with the same leaf, and the spelling alone sent it to
/// the `Result` writer: `o.isOk()` and `o.unwrap()` on a class that carries
/// `is`, `value` and `intoMatch` and neither of those, so the call threw and
/// the enum stayed live. Identity, never spelling — the same answer S4 and I3
/// gave for `Some` and for `From`.
pub(crate) fn is_result_match(m: &syn::ExprMatch, t: &BodyTranslator) -> bool {
    m.arms.iter().any(|arm| pattern_opens_a_result(&arm.pat, &m.expr, t))
}

/// The same question for the one pattern of an `if let` or a `while let`.
pub(crate) fn pattern_opens_a_result(
    pat: &syn::Pat,
    subject: &syn::Expr,
    t: &BodyTranslator,
) -> bool {
    let spelled = cases_of(pat).into_iter().any(|case| match case {
        syn::Pat::TupleStruct(ts) => {
            let name = ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
            name == "Ok" || name == "Err"
        }
        _ => false,
    });
    spelled && !subject_declares_its_own_variants(subject, t)
}

/// Is this expression an enum the CORPUS declared — one whose `Ok` is its own
/// variant rather than the runtime wrapper's?
///
/// Asked this way round on purpose. The positive question ("did the engine
/// resolve this to `std::result::Result`") is answered NO by every subject the
/// engine reads as an alias, a projection or a foreign wrapper, and moving
/// those onto the if-chain has nothing to do with what they match: it turned
/// `storage-indexeddb/scanner.rs`'s `stream.next().await?` — a genuine
/// `Result` reached through a `Stream`'s `Item` — into `result.is('Ok')`.
pub(crate) fn subject_declares_its_own_variants(expr: &syn::Expr, t: &BodyTranslator) -> bool {
    // Asking is not translating: what this resolution defers is reported once,
    // where the match is written out.
    let mark = t.mark();
    let resolved = t.resolve_expr_type(expr).ok();
    t.rewind(mark);
    resolved.is_some_and(|ty| t.declares_its_own_variants(&ty))
}

#[cfg(test)]
mod tests {
    use super::names_a_variant;
    use crate::match_expr::arms::cases_of;

    fn any_case(src: &str, own_enum: bool) -> bool {
        let expr: syn::ExprMatch = syn::parse_str(src).expect("parses");
        expr.arms
            .iter()
            .flat_map(|arm| cases_of(&arm.pat))
            .any(|pat| names_a_variant(pat, own_enum))
    }

    #[test]
    fn an_or_pattern_is_read_as_its_alternatives() {
        assert!(any_case("match s { Shape::One(t) | Shape::Two(t, _) => 1, _ => 0 }", false));
    }

    #[test]
    fn a_wildcard_beside_an_or_of_ok_and_err_is_not_an_enum_match() {
        assert!(!any_case("match r { Ok(t) | Err(t) => 1, _ => 0 }", false));
    }

    #[test]
    fn the_same_arms_over_a_crate_enum_are_one() {
        assert!(any_case("match r { Ok(t) | Err(t) => 1, _ => 0 }", true));
    }

    #[test]
    fn a_plain_variant_arm_still_answers() {
        assert!(any_case("match s { Shape::One(t) => 1, _ => 0 }", false));
    }

    #[test]
    fn a_match_on_numbers_is_not_an_enum_match() {
        assert!(!any_case("match n { 1 | 2 => 1, _ => 0 }", false));
    }
}
