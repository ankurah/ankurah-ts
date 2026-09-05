//! `match opt { Some(..) => .., None => .. }`, written as an if-chain.
//!
//! The port writes `Option<T>` as `T | null`, so an `Option` has no `.match`
//! and its arms have to become tests. The straightforward shape — one `Some`
//! binding a name, one `None` — used to be the only one written, and everything
//! else was quietly lost: `match a.partial_cmp(b) { Some(Ordering::Greater) =>
//! true, Some(Ordering::Equal) => .., Some(Ordering::Less) => false, None =>
//! false }` kept the LAST `Some` arm, wrote its pattern where a binding name
//! belongs (`const cmp.Ordering.Less = ..`, which does not parse), evaluated
//! `a.partial_cmp(b)` twice, and reported nothing.
//!
//! Arms are tested in the order Rust tries them, each against the same subject,
//! which is read once.

use super::{Position, arm_body, indent, subject_of_bound};
use crate::body::BodyTranslator;

pub fn translate(
    scrutinee: &str,
    match_expr: &syn::ExprMatch,
    t: &BodyTranslator,
    position: Position,
) -> String {
    let scrutinee_ty = t.scrutinee_type(&match_expr.expr);
    let takes = t.match_takes(match_expr);
    // Rust evaluates the subject once and each arm tests that one value. The
    // arms are tests here, so a subject that is not already a name is read into
    // one; without it a call with side effects ran once per arm.
    // Every name the arms bind, so a pattern that shadows the subject's own
    // name gets a temporary to shadow against.
    let binds: Vec<String> = match_expr
        .arms
        .iter()
        .flat_map(|arm| crate::body::pattern_names(&arm.pat))
        .collect();
    let (subject, declaration) = subject_of_bound(scrutinee, &binds, t);

    // A CONSUMING match whose arm tests inside the payload has no lowering
    // here, and what the chain writes for one is wrong in two ways at once: the
    // names it binds out of the payload are never detached from the value they
    // came out of, so releasing that value drops them a second time; and no arm
    // releases the value at all, because the chain claims only the names each
    // arm bound. `Some(Payload::Held(token)) => sink(token)` handed `token` on
    // and leaked the `Payload` it came out of.
    //
    // `Option<T>` is `T | null`, so the value under test IS the payload: the
    // right lowering is the consuming enum match, which detaches the payload
    // and marks the enum moved. Until the arm chain can test inside a payload
    // and own it in Rust's arm order, this is a hole (R12) rather than an arm
    // that runs and answers something Rust would not.
    if takes == crate::ownership::scrutinee::Takes::Payload
        && match_expr.arms.iter().any(|arm| tests_inside_the_payload(&arm.pat))
    {
        return t.hole(
            syn::spanned::Spanned::span(match_expr),
            "an arm of this consuming `Option` match tests inside the payload, and the port \
             cannot both take a name out of that payload and release what is left of it here",
        ) + ";";
    }

    let mut branches: Vec<(String, String)> = Vec::new();
    let mut otherwise: Option<String> = None;
    // Rust's match is exhaustive, so if control reaches the last arm's test that
    // arm matches — which is why the last branch is written as a plain `else`,
    // and TypeScript can then see that the chain always produces a value. That
    // holds only while every arm was written; an arm the engine gave up on is
    // reported here and takes the shortcut with it.
    let mut wrote_every_arm = true;

    for arm in &match_expr.arms {
        if otherwise.is_some() {
            t.report_match_gap(
                match_expr,
                "an arm above this one matches every value, and Rust tries arms in order, so \
                 this one never runs and is not written",
            );
            wrote_every_arm = false;
            continue;
        }
        let (test, bind) = t.pattern_test(&subject, &arm.pat);
        let _entered = t.enter_pattern(&arm.pat, scrutinee_ty.as_ref());
        let owned = payload_owned(&arm.pat, arm, takes, t);
        // The binding stands OUTSIDE the `try`, because the `finally` that
        // releases it is a sibling of that block and cannot see a `const`
        // declared inside it. Written the other way — `try { const value = old;
        // .. } finally { value.drop(); }` — every arm that owned what it bound
        // threw `ReferenceError: value is not defined` on the way out, and then
        // leaked the value it was trying to release.
        let body = format!(
            "{}{}",
            bind,
            t.wrap_bindings(&owned, format!("{}\n", arm_body(&arm.body, t, position)))
        );
        if test == "true" {
            otherwise = Some(body);
        } else {
            branches.push((test, body));
        }
    }

    if branches.is_empty() && otherwise.is_none() {
        t.report_match_gap(
            match_expr,
            "this `Option` match has no arm the port can test, so nothing was written for it",
        );
        return format!("undefined /* match {} */;", subject);
    }

    // With no catch-all and every arm written, the last arm is the one Rust's
    // exhaustiveness leaves, so it is written as the `else`.
    if otherwise.is_none() && wrote_every_arm && branches.len() > 1 {
        otherwise = branches.pop().map(|(_, body)| body);
    }

    let mut out = declaration;
    for (i, (test, body)) in branches.iter().enumerate() {
        let head = if i == 0 { "if" } else { "} else if" };
        out.push_str(&format!("{} ({}) {{\n{}", head, test, indent(body)));
    }
    match (&otherwise, branches.is_empty()) {
        // Every arm tested; the value that matched none of them falls through.
        (None, false) => out.push('}'),
        (Some(body), false) => out.push_str(&format!("}} else {{\n{}}}", indent(body))),
        // Only a catch-all: no test to write, so the body stands on its own.
        (Some(body), true) => out.push_str(body),
        (None, true) => {}
    }
    out
}

/// What an arm owns of the payload it was handed.
///
/// `Option<T>` is `T | null`, so the payload IS the subject and every name the
/// arm's pattern binds is another name for it. Where the match consumes, the arm
/// owns those names for its own length and releases them however it is left.
fn payload_owned(
    pat: &syn::Pat,
    arm: &syn::Arm,
    takes: crate::ownership::scrutinee::Takes,
    t: &BodyTranslator,
) -> Vec<crate::ownership::Owned> {
    if takes != crate::ownership::scrutinee::Takes::Payload {
        return Vec::new();
    }
    let names: Vec<String> = crate::body::pattern_names(pat)
        .into_iter()
        .filter(|n| n != "_")
        .collect();
    if names.is_empty() {
        return Vec::new();
    }
    t.claim_bindings(
        &names,
        std::slice::from_ref(&syn::Stmt::Expr(arm.body.as_ref().clone(), None)),
    )
}

/// Does this arm's pattern look INSIDE the `Option`'s payload?
///
/// `Some(p)` and `Some(_)` take the payload as it stands; `Some(Payload::Held(t))`
/// and `Some(Wrap { n })` test what is in it and bind out of it, which is the
/// shape the chain has no ownership lowering for.
fn tests_inside_the_payload(pat: &syn::Pat) -> bool {
    let syn::Pat::TupleStruct(ts) = pat else { return false };
    if ts.path.segments.last().map(|s| s.ident.to_string()).as_deref() != Some("Some") {
        return false;
    }
    let Some(inner) = ts.elems.first() else { return false };
    !matches!(inner, syn::Pat::Wild(_) | syn::Pat::Ident(syn::PatIdent { subpat: None, .. }))
}

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

    const ENUM: &str = "pub struct Token { pub n: usize }\n\
                        pub enum Payload { Held(Token), Free }\n\
                        pub fn sink(token: Token) -> usize { token.n }\n\
                        pub fn hold(p: Payload) -> usize { 7 }\n";

    fn built(src: &str) -> Fixture {
        Fixture::build(&[("lib.rs", &format!("{}{}", ENUM, src))])
    }

    /// A `finally` is a SIBLING of the `try` beside it, so a `const` declared
    /// inside the block is not a name the release can see. Written the other
    /// way this threw `ReferenceError: bounds is not defined` on the way out of
    /// every arm that owned what it bound — live at storage-common's
    /// `planner.rs`, where the release read `if (!_moved2) bounds.drop()`.
    #[test]
    fn a_binding_the_arm_releases_is_declared_outside_the_try() {
        let mut f = built(
            "pub fn read(slot: Option<Payload>) -> usize {\n\
               match slot { Some(p) => 1, None => 0 }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "read");
        let bind = ts.find("const p = slot;").unwrap_or_else(|| panic!("{}", ts));
        let opened = ts.find("try {").unwrap_or_else(|| panic!("{}", ts));
        assert!(bind < opened, "the binding stands before the try that releases it:\n{}", ts);
        assert!(ts.contains("p.drop();"), "{}", ts);
    }

    /// The payload handed on whole owes nothing: no release, and so no `try`.
    #[test]
    fn a_payload_handed_on_leaves_nothing_to_release() {
        let mut f = built(
            "pub fn whole(slot: Option<Payload>) -> usize {\n\
               match slot { Some(p) => hold(p), None => 0 }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "whole");
        assert!(ts.contains("const p = slot;"), "{}", ts);
        assert!(!ts.contains("try {"), "nothing is owed, so nothing is wrapped:\n{}", ts);
    }

    /// An arm that tests inside the payload has no lowering here: the name it
    /// binds out of the payload is never detached from it, and no arm releases
    /// what is left. It is a hole, not an arm that runs (R12).
    #[test]
    fn an_arm_that_tests_inside_a_consumed_payload_is_a_hole() {
        let mut f = built(
            "pub fn nested(slot: Option<Payload>) -> usize {\n\
               match slot {\n\
                 Some(Payload::Held(token)) => sink(token),\n\
                 Some(Payload::Free) => 1,\n\
                 None => 0,\n\
               }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "nested");
        assert!(ts.contains("unsupported("), "{}", ts);
        assert!(!ts.contains("slot.value"), "no arm of it is written at all:\n{}", ts);
        assert!(
            f.messages().iter().any(|m| m.contains("tests inside the payload")),
            "and the gap is reported: {:?}",
            f.messages()
        );
    }

    /// A BORROWING match of the same shape is untouched: nothing is taken out
    /// of the payload, so there is nothing to detach and nothing to release.
    #[test]
    fn an_arm_that_tests_inside_a_borrowed_payload_is_still_written() {
        let mut f = built(
            "pub fn peek(slot: &Option<Payload>) -> usize {\n\
               match slot {\n\
                 Some(Payload::Held(token)) => token.n,\n\
                 Some(Payload::Free) => 1,\n\
                 None => 0,\n\
               }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "peek");
        assert!(!ts.contains("unsupported("), "{}", ts);
        assert!(ts.contains("is('Held')"), "{}", ts);
    }
}
