//! A variant several arms name, or one an arm names without covering it.
//!
//! For: Rust tries a match's arms in ORDER, testing the patterns inside the
//! payload; the runtime's `.match({..})` has one key per variant and dispatches
//! on the variant name alone. Where two arms name one variant — `Expr::Literal(
//! Literal::Bool(true))` beside `..(false)` — the key held one of them and that
//! one ran for every value of the variant: ankql's `Predicate::try_from` turned
//! `FALSE` into `TRUE`, and `Poll::Ready(Some(item))` beside `Poll::Ready(None)`
//! read the end of a stream as an item.
//!
//! What is written instead is ONE key whose body tries those arms in Rust
//! order: each arm's inner pattern is a test, its own bindings and body are the
//! branch the test guards, and a value no arm matched falls through to the
//! catch-all's body. The payload arrives once, as the key's parameter, and
//! whichever branch runs is what settles it — the branch releases the names its
//! pattern took and `dropUnbound`s the rest, exactly as a single arm does.

use super::arms;
use super::catch_all;
use crate::body::{indent, BodyTranslator};

/// One arm of the chain.
pub(super) struct Link {
    /// The test the arm's inner pattern makes of the payload, or nothing where
    /// the pattern matches every value of the variant.
    pub test: Option<String>,
    /// Everything that runs when the test passes: the drop flags the body owes,
    /// the names the pattern takes out of the payload, and the body.
    pub block: String,
    /// Whether the branch awaits, which makes the key `async` and the match
    /// around it awaited.
    pub is_async: bool,
}

/// What the chain runs when no arm's pattern matched.
pub(super) enum Fallthrough<'a> {
    /// The match has no arm that matches anything, so rustc proved these arms
    /// exhaustive between them — `Some(x)` and `None` cover an `Option`, and
    /// neither covers it alone.
    Exhaustive,
    /// The catch-all, which the chain asks for the body of under the variant it
    /// is writing.
    CatchAll(&'a catch_all::Fallback<'a>),
    /// A catch-all whose body the port cannot write here, with the reason.
    Unwritable(String),
}

/// The variants this match cannot write as one key each.
///
/// Two arms naming one variant is the loud case. One arm naming a variant it
/// does not COVER is the same gap read from the other side: the values its
/// inner pattern rejects belong to the catch-all, and a key with no test hands
/// them to the arm anyway.
pub(super) fn contested(arms_of: &[&syn::Arm], has_catch_all: bool) -> Vec<String> {
    let mut order: Vec<String> = Vec::new();
    let mut seen: Vec<(String, usize, bool)> = Vec::new();
    for arm in arms_of.iter().copied() {
        for case in arms::cases_of(&arm.pat) {
            let Some((variant, _)) = arms::payload_of(case) else { continue };
            let covers = catch_all::covers_its_variant(case);
            match seen.iter_mut().find(|(name, _, _)| *name == variant) {
                Some(entry) => {
                    entry.1 += 1;
                    entry.2 |= covers;
                }
                None => {
                    order.push(variant.clone());
                    seen.push((variant, 1, covers));
                }
            }
        }
    }
    order
        .into_iter()
        .filter(|name| {
            let (_, times, covered) =
                seen.iter().find(|(n, _, _)| n == name).expect("built from the same walk");
            *times > 1 || (!*covered && has_catch_all)
        })
        .collect()
}

/// What one link tests of the payload it was handed, and what it takes out of
/// it.
///
/// `arms::arm_declarations` asks the same question for a key that has no test
/// to make: it takes the names out and reports the test it cannot make. Here
/// the test is made, so it is kept. Nothing comes back for a pattern the
/// translator cannot read back — `pattern_test` writes a HOLE for an alternation
/// whose parts bind different names — because a link whose test throws is not a
/// link, and a chain that reaches it stops there.
pub(super) fn conditions(
    pat: &syn::Pat,
    param: &str,
    fields: &[(String, String)],
    t: &BodyTranslator,
) -> Option<(Option<String>, String, Vec<String>, Vec<String>)> {
    let subpats: Vec<&syn::Pat> = match pat {
        syn::Pat::TupleStruct(ts) => ts.elems.iter().collect(),
        syn::Pat::Struct(st) => st.fields.iter().map(|f| &*f.pat).collect(),
        _ => Vec::new(),
    };
    let mut tests: Vec<String> = Vec::new();
    let mut text = String::new();
    let mut bound_keys = Vec::new();
    let mut names = Vec::new();
    for (i, (local, accessor)) in fields.iter().enumerate() {
        let place = format!("{}.{}", param, accessor);
        match subpats.get(i) {
            Some(sub) if !BodyTranslator::is_irrefutable(sub) => {
                let (test, bind) = t.pattern_test(&place, sub);
                if test.starts_with("unsupported(") {
                    return None;
                }
                if test.trim() != "true" {
                    tests.push(test);
                }
                text.push_str(&bind);
                bound_keys.push(accessor.clone());
                names.extend(crate::body::pattern_names(sub));
            }
            _ => {
                if local == "_" {
                    continue;
                }
                text.push_str(&format!("const {} = {};\n", local, place));
                bound_keys.push(accessor.clone());
                names.push(local.clone());
            }
        }
    }
    // One test stands as it is; several are joined, and each is parenthesised
    // so that an `||` inside one — which is what an alternation writes — cannot
    // reach across the `&&`.
    let test = match tests.len() {
        0 => None,
        1 => Some(tests.remove(0)),
        _ => Some(tests.iter().map(|test| format!("({})", test)).collect::<Vec<_>>().join(" && ")),
    };
    Some((test, text, bound_keys, names))
}

/// A name for the parameter each contested variant's arms share.
///
/// Chosen before any of those arms is translated, because the bindings are
/// written out of it — and chosen against every body in the chain, the
/// catch-all's included, because they are all branches of one arrow.
pub(super) fn parameters(
    written_arms: &[&syn::Arm],
    contested: &[String],
    fall: &Fallthrough<'_>,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for variant in contested {
        let mut fields: Vec<(String, String)> = Vec::new();
        let mut bodies: Vec<&syn::Expr> = Vec::new();
        for arm in written_arms.iter().copied() {
            for case in arms::cases_of(&arm.pat) {
                let Some((name, found)) = arms::payload_of(case) else { continue };
                if &name != variant {
                    continue;
                }
                fields.extend(found);
                bodies.push(&arm.body);
            }
        }
        if let Fallthrough::CatchAll(fallback) = fall {
            bodies.push(fallback.rest_body);
        }
        out.push((variant.clone(), arms::shared_parameter(&fields, &bodies)));
    }
    out
}

/// The chain, as the body of the one key the runtime's match dispatches to.
pub(super) fn write(
    links: Vec<Link>,
    fall: &Fallthrough<'_>,
    variant: &str,
    has_payload: bool,
    param: &str,
    t: &BodyTranslator,
) -> String {
    // Rust never reads past an arm that matches every value of the variant, so
    // neither does this: the arms below such a link are unreachable, and
    // rustc's own "unreachable pattern" is what says so about the source.
    let mut parts: Vec<(Option<String>, String)> = Vec::new();
    for link in links {
        let unconditional = link.test.is_none();
        parts.push((link.test, link.block));
        if unconditional {
            break;
        }
    }
    let open = parts.last().map(|(test, _)| test.is_some()).unwrap_or(true);
    if open {
        match fall {
            Fallthrough::CatchAll(fallback) => {
                parts.push((None, fallback.statements(variant, has_payload, param, t)))
            }
            Fallthrough::Unwritable(why) => parts.push((None, unwritable(why))),
            // rustc proved the arms exhaustive between them, so a value that
            // failed every test above matches the last of them.
            Fallthrough::Exhaustive => {
                if let Some(last) = parts.last_mut() {
                    last.0 = None;
                }
            }
        }
    }

    // One branch and no test is the whole body: the chain collapsed to the arm
    // it started as.
    if parts.len() == 1 && parts[0].0.is_none() {
        return parts.remove(0).1;
    }
    let mut out = String::new();
    for (i, (test, block)) in parts.iter().enumerate() {
        match (i, test) {
            (0, Some(test)) => out.push_str(&format!("if ({}) {{\n", test)),
            (_, Some(test)) => out.push_str(&format!("}} else if ({}) {{\n", test)),
            (0, None) => unreachable!("a chain of one unconditional branch is written above"),
            (_, None) => out.push_str("} else {\n"),
        }
        out.push_str(&indent(block));
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    out.push_str("}\n");
    out
}

/// The chain's `else` where the port cannot write what belongs there.
///
/// R12: a branch whose emission is known wrong carries a hole rather than a
/// wrong answer, and the hole throws where the branch would have run.
fn unwritable(why: &str) -> String {
    format!("{};\n", crate::body::hole_text(why))
}

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

    const TYPES: &str = "pub struct Payload { pub n: u32 }\n\
         pub enum Lit { Flag(bool), Count(u32) }\n\
         pub enum Ex { Literal(Lit), Held(Payload), Nothing }\n";

    fn built(src: &str) -> Fixture {
        Fixture::build(&[("lib.rs", &format!("{}{}", TYPES, src))])
    }

    /// The arms are tried in the order the source wrote them, and the
    /// catch-all's body is what a value no arm matched runs.
    #[test]
    fn the_arms_of_a_contested_variant_are_tried_in_rust_order() {
        let mut f = built(
            "pub fn truthy(e: Ex) -> u32 {\n\
               match e {\n\
                 Ex::Literal(Lit::Flag(true)) => 1,\n\
                 Ex::Literal(Lit::Flag(false)) => 2,\n\
                 _ => 3,\n\
               }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "truthy");
        let first = ts.find("=== true").expect("the first arm's test");
        let second = ts.find("=== false").expect("the second arm's test");
        let last = ts.find("3").expect("the catch-all's body");
        assert!(first < second, "the arms keep their order:\n{}", ts);
        assert!(second < last, "and the catch-all is the last else:\n{}", ts);
        assert!(ts.contains("} else {"), "{}", ts);
        assert!(
            !ts.contains("unsupported("),
            "the hole this replaces is gone:\n{}",
            ts
        );
    }

    /// With no catch-all, rustc proved the arms exhaustive between them, so a
    /// value that failed every test above matches the last of them and its test
    /// is not written.
    #[test]
    fn the_last_arm_of_an_exhaustive_chain_needs_no_test() {
        let mut f = built(
            "pub fn describe(e: &Ex) -> u32 {\n\
               match e {\n\
                 Ex::Literal(Lit::Flag(_)) => 1,\n\
                 Ex::Literal(Lit::Count(_)) => 2,\n\
                 Ex::Held(_) => 3,\n\
                 Ex::Nothing => 4,\n\
               }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "describe");
        assert!(ts.contains("if (v._0.is('Flag')) {"), "{}", ts);
        assert!(ts.contains("} else {\n"), "{}", ts);
        assert!(!ts.contains("is('Count')"), "the last arm needs no test:\n{}", ts);
    }

    /// The subject is evaluated once. Rust evaluates it once, and a subject
    /// written twice would run its side effects twice — which is what a chain
    /// spelled as a second `.match` on the same expression would do.
    #[test]
    fn a_subject_with_side_effects_is_written_once() {
        let mut f = built(
            "pub fn pop(items: &mut Vec<Ex>) -> Ex { items.pop().unwrap() }\n\
             pub fn first(items: &mut Vec<Ex>) -> u32 {\n\
               match pop(items) {\n\
                 Ex::Literal(Lit::Flag(true)) => 1,\n\
                 Ex::Literal(Lit::Flag(false)) => 2,\n\
                 _ => 3,\n\
               }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "first");
        assert_eq!(ts.matches("pop(items)").count(), 1, "the subject is read once:\n{}", ts);
    }

    /// R12: an arm whose pattern the translator cannot read back is a hole from
    /// that arm on, rather than a body run for every value of the variant.
    #[test]
    fn an_arm_with_a_guard_is_a_hole_from_there_on() {
        let mut f = built(
            "pub fn pick(e: Ex, limit: u32) -> u32 {\n\
               match e {\n\
                 Ex::Literal(Lit::Count(n)) if n > limit => 1,\n\
                 Ex::Literal(Lit::Count(_)) => 2,\n\
                 _ => 3,\n\
               }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "pick");
        assert!(ts.contains("unsupported("), "{}", ts);
        assert!(
            f.messages().iter().any(|m| m.contains("has a guard")),
            "and it says why: {:?}",
            f.messages()
        );
    }

    /// The chain's arms are branches of ONE arrow, so they share one parameter,
    /// and a `let` in any of their bodies — or in the catch-all's — must not be
    /// declared beside it.
    #[test]
    fn the_shared_parameter_avoids_every_name_the_branches_declare() {
        let mut f = built(
            "pub fn pick(e: Ex) -> u32 {\n\
               match e {\n\
                 Ex::Literal(Lit::Flag(true)) => 1,\n\
                 Ex::Literal(Lit::Count(_)) => { let v = 8; v }\n\
                 Ex::Held(p) => p.n,\n\
                 _ => 3,\n\
               }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "pick");
        assert!(ts.contains("Literal: (_v) =>"), "{}", ts);
        assert!(ts.contains("const v = 8;"), "{}", ts);
    }
}
