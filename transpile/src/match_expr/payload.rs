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
    let mut found = Vec::new();
    let mut out = Payload {
        test: None,
        text: String::new(),
        bound_keys: Vec::new(),
        names: Vec::new(),
    };
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
                // `bound_keys` is what `dropUnbound` EXCLUDES: the parts of the
                // payload some name has taken, and which the body therefore
                // releases itself. A subpattern that only tests — `Lit::Flag(_)`
                // — takes nothing, so listing its member here excluded the only
                // key there was and the link released nothing at all.
                // ... and a subpattern that reaches INSIDE the member without
                // taking anything droppable out of it takes nothing either:
                // `Expr::Literal(Lit::Count(n))` binds a `u32` and leaves the
                // `Lit` whole, so listing `_0` here excluded the one member
                // nobody owned. That is the nested-payload wrapper leak.
                if !BodyTranslator::binds_nothing(sub) && !super::owing::member_is_left_whole(sub, t)
                {
                    out.bound_keys.push(accessor.clone());
                }
                out.names.extend(crate::body::pattern_names(sub));
            }
            _ => {
                if local == "_" {
                    continue;
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
