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

use super::{Position, arm_body, indent, subject_of};
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
    let (subject, declaration) = subject_of(scrutinee, t);

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
        let body = t.wrap_bindings(&owned, format!("{}{}\n", bind, arm_body(&arm.body, t, position)));
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
