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
    /// The drop flags the body owes and the names the pattern takes out of the
    /// payload, which stand before the guard because the guard reads them.
    pub bindings: String,
    /// The arm's SECOND test, made after the pattern's and after the names it
    /// bound are in scope. A guard that fails hands the payload to the arm
    /// below, which is why a chain with one is written as a run of `if`s rather
    /// than an `else if` chain.
    pub guard: Option<String>,
    /// Everything that runs when both tests pass: the body and what it owes
    /// around it.
    pub block: String,
    /// Whether the block leaves the arrow by itself, so that nothing written
    /// after it in the chain would run. A block that does not needs a `break`
    /// out of the chain's label, or the arms below it would be tried after it.
    pub leaves: bool,
    /// Whether the branch awaits, which makes the key `async` and the match
    /// around it awaited.
    pub is_async: bool,
}

impl Link {
    /// The link as one piece of text, for the forms that need no fall-through.
    fn whole(&self) -> String {
        format!("{}{}", self.bindings, self.block)
    }
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
    // variant → (how many arms name it, whether any of them covers it, whether
    // any of them carries a guard).
    let mut seen: Vec<(String, usize, bool, bool)> = Vec::new();
    for arm in arms_of.iter().copied() {
        for case in arms::cases_of(&arm.pat) {
            let Some((variant, _)) = arms::payload_of(case) else { continue };
            // A GUARDED arm covers nothing: the values whose guard fails belong
            // to the arm below it, and a key with no test of its own hands them
            // to this one anyway. So a guard puts its variant on the chain,
            // which is the only form that can try the next arm.
            let guarded = arm.guard.is_some();
            let covers = !guarded && catch_all::covers_its_variant(case);
            match seen.iter_mut().find(|(name, _, _, _)| *name == variant) {
                Some(entry) => {
                    entry.1 += 1;
                    entry.2 |= covers;
                    entry.3 |= guarded;
                }
                None => {
                    order.push(variant.clone());
                    seen.push((variant, 1, covers, guarded));
                }
            }
        }
    }
    order
        .into_iter()
        .filter(|name| {
            let (_, times, covered, guarded) =
                seen.iter().find(|(n, _, _, _)| n == name).expect("built from the same walk");
            *times > 1 || *guarded || (!*covered && has_catch_all)
        })
        .collect()
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
    takes: crate::ownership::scrutinee::Takes,
    t: &BodyTranslator,
) -> String {
    // A guard is a test the port cannot write where the pattern's test goes,
    // because it reads the names the pattern binds — so a chain with one is
    // written as a run of `if`s inside a labelled block, and a link that ran
    // leaves the block rather than falling into the arms below it.
    if links.iter().any(|link| link.guard.is_some()) {
        return in_turn(links, fall, variant, has_payload, param, takes, t);
    }

    // Rust never reads past an arm that matches every value of the variant, so
    // neither does this: the arms below such a link are unreachable, and
    // rustc's own "unreachable pattern" is what says so about the source.
    let mut parts: Vec<(Option<String>, String)> = Vec::new();
    for link in links {
        let unconditional = link.test.is_none();
        let whole = link.whole();
        parts.push((link.test, whole));
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
            Fallthrough::Unwritable(why) => {
                parts.push((None, arms::hole_in_an_arm(why, param, has_payload, takes)))
            }
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

/// The chain when one of its arms carries a GUARD.
///
/// A guard reads the names its own pattern bound, and those names are declared
/// inside the branch the pattern's test opens — so the guard cannot be part of
/// that test, and an `else if` chain has nowhere to put it. What Rust does when
/// a guard fails is try the NEXT arm, with the value untouched, so that is what
/// this writes: each arm is an `if` of its own, standing one after another
/// inside a labelled block, and an arm that ran leaves the block instead of
/// falling into the arms below it.
///
/// ```text
/// $match0: {
///   if (v.is('Flag')) {
///     const b = v.value._0;
///     if (b) { … ; break $match0; }
///   }
///   … the next arm …
///   … the catch-all's body …
/// }
/// ```
///
/// A body that returns or throws has made that jump for itself, so it gets no
/// `break`.
fn in_turn(
    links: Vec<Link>,
    fall: &Fallthrough<'_>,
    variant: &str,
    has_payload: bool,
    param: &str,
    takes: crate::ownership::scrutinee::Takes,
    t: &BodyTranslator,
) -> String {
    // An arm with neither a test nor a guard matches every value of the
    // variant, so Rust never reads past it and neither does this.
    let unconditional = |link: &Link| link.test.is_none() && link.guard.is_none();
    let mut open = true;
    let mut kept: Vec<Link> = Vec::new();
    for link in links {
        let last = unconditional(&link);
        kept.push(link);
        if last {
            open = false;
            break;
        }
    }
    // A label nothing jumps to is noise: where every arm's body returns or
    // throws by itself, the chain needs no way out.
    let needs_break = kept.iter().enumerate().any(|(i, link)| {
        !link.leaves && !unconditional(link) && (open || i + 1 < kept.len())
    });
    let label = if needs_break { t.fresh_hoist("_match") } else { String::new() };
    let mut inner = String::new();
    for link in kept {
        let unconditional = unconditional(&link);
        let leaving = if link.leaves || unconditional || label.is_empty() {
            String::new()
        } else {
            format!("break {};\n", label)
        };
        let guarded = match &link.guard {
            Some(guard) => format!(
                "{}if ({}) {{\n{}}}\n",
                link.bindings,
                guard,
                indent(&format!("{}{}", link.block, leaving))
            ),
            None => format!("{}{}{}", link.bindings, link.block, leaving),
        };
        match &link.test {
            Some(test) => inner.push_str(&format!("if ({}) {{\n{}}}\n", test, indent(&guarded))),
            // A pattern that matches every value of the variant still opens a
            // block of its own: the names it binds belong to this arm and to
            // no arm written after it.
            None => inner.push_str(&format!("{{\n{}}}\n", indent(&guarded))),
        }
    }
    if open {
        match fall {
            Fallthrough::CatchAll(fallback) => {
                inner.push_str(&fallback.statements(variant, has_payload, param, t))
            }
            Fallthrough::Unwritable(why) => {
                inner.push_str(&arms::hole_in_an_arm(why, param, has_payload, takes))
            }
            // Every arm here is conditional — a guard makes even an
            // irrefutable pattern one — so an exhaustive match still needs
            // something written where a failed guard lands. Rust proves it
            // unreachable; the port cannot, and a chain that fell off its end
            // would hand back `undefined`.
            Fallthrough::Exhaustive => inner.push_str(&arms::hole_in_an_arm(
                &format!(
                    "every arm naming `{}` has a guard, and rustc proved between them that one \
                     always holds; the port cannot see that proof, so a value that fails all of \
                     them arrives here",
                    variant
                ),
                param,
                has_payload,
                takes,
            )),
        }
    }
    if label.is_empty() {
        return inner;
    }
    format!("{}: {{\n{}}}\n", label, indent(&inner))
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

    /// Y1: a link that REFUSES still owns the payload the arm was handed, and
    /// `intoMatch` releases nothing of its own on any path out. Before the sixth
    /// pass the refusal threw and left the whole payload to nobody.
    #[test]
    fn a_hole_in_a_consuming_link_releases_the_payload_before_it_throws() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct Token { pub n: u32 }\n\
             pub enum Inner { A((Token, Token)), B((Token, Token)) }\n\
             pub enum Wrap { Held(Inner, Token), Empty }\n\
             pub fn pick(w: Wrap) -> u32 {\n\
               match w {\n\
                 Wrap::Held(Inner::A((a, b)) | Inner::B((b, a)), _) => \
                   { let n = a.n + b.n; drop(a); drop(b); n }\n\
                 Wrap::Held(_, rest) => { let n = rest.n; drop(rest); n }\n\
                 Wrap::Empty => 0,\n\
               }\n\
             }",
        )]);
        let ts = f.translated_method("lib.rs", "pick");
        let release = ts.find("dropUnbound(v, []);").expect(&ts);
        let throw = ts.find("unsupported(").expect(&ts);
        assert!(release < throw, "the refusal releases what it holds first:\n{}", ts);
        // D2: the TEST still decides, so a value the refusing arm does not
        // match reaches the arm below it.
        assert!(ts.contains("v._0.is('A')"), "the test is written:\n{}", ts);
    }

    /// And a BORROWED link's refusal owes nothing: the subject is still its
    /// owner's, and a release here would be a second one.
    #[test]
    fn a_hole_in_a_borrowed_link_releases_nothing() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct Token { pub n: u32 }\n\
             pub enum Inner { A((Token, Token)), B((Token, Token)) }\n\
             pub enum Wrap { Held(Inner, Token), Empty }\n\
             pub fn pick(w: &Wrap) -> u32 {\n\
               match w {\n\
                 Wrap::Held(Inner::A((a, b)) | Inner::B((b, a)), _) => a.n + b.n,\n\
                 Wrap::Held(_, rest) => rest.n,\n\
                 Wrap::Empty => 0,\n\
               }\n\
             }",
        )]);
        let ts = f.translated_method("lib.rs", "pick");
        assert!(ts.contains("unsupported("), "{}", ts);
        assert!(!ts.contains("dropUnbound"), "a borrowed link owes nothing:\n{}", ts);
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
    fn an_arm_with_a_guard_is_tried_and_falls_through_to_the_next() {
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
        // PREMISE CHANGED 2026-09-05 (fixpass6 item 2, D8): a guard used to
        // make this arm and the arms below it a hole. The chain writes it now:
        // the pattern's test opens a block, the names it bound stand in that
        // block, the guard is tested there, and a guard that fails falls
        // through to the arm below.
        assert!(!ts.contains("unsupported("), "{}", ts);
        let guard = ts.find("if (n > limit)").expect(&ts);
        let below = ts.find("return 2;").expect(&ts);
        assert!(guard < below, "a failed guard reaches the arm below it:\n{}", ts);
        assert!(
            f.messages().iter().all(|m| !m.contains("guard is dropped")),
            "and nothing says the guard was dropped: {:?}",
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
