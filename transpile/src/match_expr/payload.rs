//! What a variant's PAYLOAD contributes to the arm that names it.
//!
//! For: two callers ask the same question of the same fields — a chain link,
//! which writes the inner test because the arms below it are there to fall
//! through to, and a key that stands alone, which has no later arm and so
//! cannot make the test at all. They were forty lines apiece, differing in
//! those two lines, and the second copy kept the D1 defect the first had fixed.
//! One walk, told which of the two it is.

use crate::body::BodyTranslator;

/// What one link takes out of the payload it was handed, and what it asks of it.
///
/// One walk over a variant's fields, for both the callers that need one. A
/// CHAIN link writes the test — `Tests::Kept` — because the arms below it are
/// there to fall through to. A key that stands alone has no later arm, so it
/// cannot make the test at all and the site says so — `Tests::Reported`. They
/// were forty lines apiece, differing in those two lines, and the second copy
/// kept the D1 defect the first had fixed: an accessor pushed into `bound_keys`
/// for a subpattern that takes no name, which excludes from `dropUnbound` a key
/// nothing released.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Tests {
    Kept,
    Reported,
}

/// What the walk found.
pub(super) struct Payload {
    /// The test the inner patterns make, where the caller keeps it.
    pub test: Option<String>,
    /// The declarations the arm's body reads its names from.
    pub text: String,
    /// The members some name TOOK, which `dropUnbound` excludes.
    pub bound_keys: Vec<String>,
    /// Every name the pattern binds.
    pub names: Vec<String>,
    /// Why this arm's BODY is a hole, where the pattern took a droppable name
    /// out of a member and left the rest (K4). The TEST still stands, so a
    /// value this pattern does not match reaches the arms below it.
    pub refused: Option<String>,
}

/// Nothing comes back for a pattern the translator cannot read back —
/// `pattern_test` writes a HOLE for an alternation whose parts bind different
/// names — because a link whose test throws is not a link, and a chain that
/// reaches it stops there. Only `Tests::Kept` can refuse: a reported walk has no
/// test to be a hole.
pub(super) fn payload_walk(
    pat: &syn::Pat,
    param: &str,
    fields: &[(String, String)],
    t: &BodyTranslator,
    tests: Tests,
    match_expr: &syn::ExprMatch,
) -> Option<Payload> {
    let subpats: Vec<&syn::Pat> = match pat {
        syn::Pat::TupleStruct(ts) => ts.elems.iter().collect(),
        syn::Pat::Struct(st) => st.fields.iter().map(|f| &*f.pat).collect(),
        _ => Vec::new(),
    };
    // K9: `..` stands for the members the pattern did not name, and each
    // element takes the member at its OWN position — so a `..` anywhere but
    // last shifts every member after it. `Variant(.., b)` would bind `b` from
    // `_1` where the variant's last member may be `_5`, which is a name bound
    // from the wrong place and never says so. No corpus site writes one.
    let rest_out_of_place = subpats
        .iter()
        .position(|p| matches!(p, syn::Pat::Rest(_)))
        .is_some_and(|at| at + 1 != subpats.len());
    let mut found = Vec::new();
    let mut out = Payload {
        test: None,
        text: String::new(),
        bound_keys: Vec::new(),
        names: Vec::new(),
        refused: None,
    };
    if rest_out_of_place {
        let what = "a `..` written before the last element of this pattern stands for the \
                    members between it and the ones named after it, and each element here takes \
                    the member at its own position — so every name after the `..` would be bound \
                    from the wrong member";
        t.fallback(syn::spanned::Spanned::span(pat), what);
        out.refused = Some(what.to_string());
    }
    for (i, (local, accessor)) in fields.iter().enumerate() {
        let place = format!("{}.{}", param, accessor);
        match subpats.get(i) {
            Some(sub) if !BodyTranslator::is_irrefutable(sub) => {
                let (test, bind) = t.pattern_test(&place, sub);
                match tests {
                    Tests::Kept => {
                        if test.starts_with("unsupported(") {
                            return None;
                        }
                        if test.trim() != "true" {
                            found.push(test);
                        }
                    }
                    Tests::Reported => t.report_match_gap(
                        match_expr,
                        format!(
                            "this arm tests inside the payload of `{}`, and the runtime's match \
                             dispatches on the variant alone with no later arm to fall through \
                             to, so the inner test is not made and the arm runs for every `{}`",
                            accessor, accessor
                        ),
                    ),
                }
                out.text.push_str(&bind);
                // `bound_keys` is what `dropUnbound` EXCLUDES: the members some
                // name has taken, and which the body therefore releases itself.
                // A subpattern that only tests — `Lit::Flag(_)` — takes nothing,
                // so listing its member here excluded the only key there was and
                // the link released nothing at all; and one that reaches INSIDE
                // the member without taking anything droppable out takes nothing
                // either (`Expr::Literal(Lit::Count(n))` binds a `u32` and leaves
                // the `Lit` whole). K4: a subpattern that takes a DROPPABLE name
                // out of the member is the third case, and it is neither of
                // these — the member is partly moved, the port cannot release an
                // object minus a field, and the `Result` side has always refused
                // it. Here it used to be excluded from the release in silence,
                // so whatever the pattern did not take leaked.
                match super::taking::taken(sub, t) {
                    super::taking::Takes::Whole => out.bound_keys.push(accessor.clone()),
                    super::taking::Takes::Nothing | super::taking::Takes::Inside => {}
                    super::taking::Takes::Part => {
                        let what = format!(
                            "this arm tests inside `{}` and takes a DROPPABLE name out of it, \
                             and the port cannot both take a name out of a payload member \
                             and release what is left of it",
                            accessor
                        );
                        t.fallback(syn::spanned::Spanned::span(sub), what.clone());
                        out.refused = Some(what);
                    }
                }
                out.names.extend(crate::body::pattern_names(sub));
            }
            _ => {
                // A member the pattern asks nothing of and takes no name out
                // of is not declared: `_` has no spelling in TypeScript, and
                // `..` (K9) covers whatever members the pattern did not name —
                // `const ... = v._0;` is not a declaration a JavaScript engine
                // will read. Neither goes into `bound_keys` either, so the arm
                // releases what it did not take.
                if local == "_"
                    || subpats.get(i).is_some_and(|sub| BodyTranslator::binds_nothing(sub))
                {
                    continue;
                }
                // An irrefutable pattern still has to say whether it took the
                // whole member. `Holder::Pair((a, _))` destructures as
                // `const [a, ] = v._0;` — nothing to test, and the member was
                // listed as bound, so `dropUnbound` skipped it and the element
                // the pattern did not name was released by nobody (H2). A tuple
                // that names every element does take the member whole.
                if let Some(sub) = subpats.get(i) {
                    if super::taking::taken(sub, t) == super::taking::Takes::Part {
                        let what = format!(
                            "this arm takes only SOME of the elements of `{}` and leaves a \
                             droppable one unnamed, and the port cannot release a tuple minus \
                             the elements a name has taken",
                            accessor
                        );
                        t.fallback(syn::spanned::Spanned::span(*sub), what.clone());
                        out.refused = Some(what);
                    }
                }
                out.text.push_str(&format!("const {} = {};\n", local, place));
                out.bound_keys.push(accessor.clone());
                out.names.push(local.clone());
            }
        }
    }
    // One test stands as it is; several are joined, and each is parenthesised
    // so that an `||` inside one — which is what an alternation writes — cannot
    // reach across the `&&`.
    out.test = match found.len() {
        0 => None,
        1 => Some(found.remove(0)),
        _ => Some(found.iter().map(|test| format!("({})", test)).collect::<Vec<_>>().join(" && ")),
    };
    Some(out)
}
