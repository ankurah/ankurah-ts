//! The arm that runs for everything the arms above it did not name.
//!
//! For: `_ => ..` and `other => ..` are how Rust keeps a match exhaustive
//! without writing every variant out, and the runtime's `match` dispatches on
//! the variant name alone — so an arm that names no variant has no key to be
//! written under. Leaving it out is what made the emitted match non-exhaustive,
//! and the runtime's fatal then fired on every value the written arms did not
//! name: on every non-tie comparison in a sort, and on every `RetrievalError`
//! that was not `AccessDenied`.
//!
//! What is written instead is one arm per variant the source did not name,
//! each carrying the catch-all's body. That keeps the emitted match exhaustive,
//! which is also what its type asks for, and it keeps the ownership the two
//! forms of the runtime match already give: `match` lends the payload, and
//! `intoMatch` hands it over and leaves the enum moved.
//!
//! The one thing an expanded arm has to put back is the *whole* value, which
//! the catch-all could name — `other => other`, `_ => Outer::Whole(err)`. Under
//! `match` the enum is still whole, so the subject stands for it. Under
//! `intoMatch` it is moved and only its payload survives, so the arm builds the
//! same value again from the variant it just matched and the payload it was
//! handed: same variant, same payload, which is what Rust's `_` arm still owns.

use crate::body::{indent, BodyTranslator};

/// A match split into the arms that name a variant and the one that does not.
pub(super) struct Split<'a> {
    /// The arms with a variant to be written under, in source order.
    pub named: Vec<&'a syn::Arm>,
    /// The arm that matches whatever is left.
    pub rest: &'a syn::Arm,
    /// The name it binds the subject to, where it binds one.
    pub binds: Option<String>,
}

/// Read a match as named arms plus a catch-all, where it has one.
///
/// Only a `_` or a bare name is a catch-all here: a name with a sub-pattern
/// (`x @ Some(_)`) asks a question, and `payload_of` writes it like any other
/// arm. Nothing comes back for a match whose arms all name a variant, which is
/// the shape the runtime's own match already expresses.
pub(super) fn split(match_expr: &syn::ExprMatch) -> Option<Split<'_>> {
    let at = match_expr.arms.iter().position(is_catch_all)?;
    let rest = &match_expr.arms[at];
    let named: Vec<&syn::Arm> = match_expr.arms[..at].iter().collect();
    if named.is_empty() {
        return None;
    }
    let binds = match &rest.pat {
        syn::Pat::Ident(ident) => Some(BodyTranslator::pat_static(&syn::Pat::Ident(ident.clone()))),
        _ => None,
    };
    Some(Split { named, rest, binds })
}

/// Is this the arm that matches anything and asks nothing?
fn is_catch_all(arm: &syn::Arm) -> bool {
    if arm.guard.is_some() {
        return false;
    }
    match &arm.pat {
        syn::Pat::Wild(_) => true,
        syn::Pat::Ident(ident) => ident.subpat.is_none(),
        _ => false,
    }
}

/// How many arms stand after the catch-all, which Rust would never reach.
pub(super) fn unreachable_after(match_expr: &syn::ExprMatch) -> usize {
    match match_expr.arms.iter().position(is_catch_all) {
        Some(at) => match_expr.arms.len() - at - 1,
        None => 0,
    }
}

/// The variants of the matched enum that no arm named, with the class the port
/// writes it as — or the reason the port cannot say.
fn remaining(
    match_expr: &syn::ExprMatch,
    split: &Split<'_>,
    t: &BodyTranslator,
) -> Result<(String, Vec<String>), String> {
    let ty = t
        .scrutinee_type(&match_expr.expr)
        .ok_or_else(|| "the engine could not type the subject".to_string())?;
    let crate::ty::Ty::Named { id, .. } = ty.peel_refs() else {
        return Err("the subject is not a named type".to_string());
    };
    let reg = t
        .registry()
        .ok_or_else(|| "this body is translated with no registry".to_string())?;
    let def = reg
        .def(*id)
        .ok_or_else(|| "the subject's type is not declared".to_string())?;
    let crate::registry::TypeKind::Enum { variants } = &def.kind else {
        return Err(format!("`{}` is not an enum", def.name));
    };
    let named: Vec<String> = split
        .named
        .iter()
        .flat_map(|arm| super::variants_of(&arm.pat))
        .collect();
    let rest: Vec<String> = variants
        .iter()
        .map(|v| v.name.clone())
        .filter(|name| !named.contains(name))
        .collect();
    Ok((def.name.clone(), rest))
}

/// The whole match, with one arm per variant the source left to the catch-all.
pub(super) fn lower(
    scrutinee: &str,
    match_expr: &syn::ExprMatch,
    split: &Split<'_>,
    t: &BodyTranslator,
    position: super::Position,
) -> String {
    let (class, rest) = match remaining(match_expr, split, t) {
        Ok(found) => found,
        Err(why) => {
            // Without the variant list there is nothing to write the arm under.
            // The named arms still stand, and the runtime's own fatal is what
            // the emitted match does with a value they do not cover.
            t.report_match_gap(
                match_expr,
                format!(
                    "an arm of this match names no variant, and the arms it stands for cannot \
                     be listed because {}, so it is not written and the emitted match is not \
                     exhaustive",
                    why
                ),
            );
            return super::enum_match_over(scrutinee, match_expr, &split.named, t, position);
        }
    };
    if rest.is_empty() {
        // Every variant is already named, so Rust's arm is unreachable and
        // rustc says as much. Writing it would be writing a branch that cannot
        // run; leaving it out changes nothing.
        return super::enum_match_over(scrutinee, match_expr, &split.named, t, position);
    }
    let takes = t.match_takes(match_expr);
    let consuming = takes == crate::ownership::scrutinee::Takes::Payload;
    let scrutinee_ty = t.scrutinee_type(&match_expr.expr);
    // The arms read the subject as well as matching on it, and Rust evaluates
    // it once, so a subject that is not already a name is read into one first.
    let named_subject = subject_name(&match_expr.expr);
    let scrutinee = match (&split.binds, &named_subject) {
        (Some(_), None) if !consuming => t.hoist_name(scrutinee.to_string()),
        _ => scrutinee.to_string(),
    };
    let scrutinee = scrutinee.as_str();

    // The name the catch-all binds owns the value from here — Rust moved it in
    // — so an arm that only reads it is what releases it, and one that hands it
    // on sets its flag instead.
    let _bindings = t.enter_pattern(&split.rest.pat, scrutinee_ty.as_ref());
    let owned = match &split.binds {
        Some(local) => t.claim_bindings(
            std::slice::from_ref(local),
            std::slice::from_ref(&syn::Stmt::Expr(split.rest.body.as_ref().clone(), None)),
        ),
        None => Vec::new(),
    };
    let (body, lifted) = t.with_own_hoists(|| super::arm_body(&split.rest.body, t, position));
    drop(_bindings);
    let body = crate::ownership::hoisted(&format!("{}\n", body.trim_end()), &lifted);
    let mut flags = match (&split.binds, consuming) {
        // A consuming match already marks the subject moved wherever it is
        // written; a borrowing one moves it only where this arm binds it.
        (Some(_), false) => t.flag_set_for_subject(&match_expr.expr),
        _ => String::new(),
    };
    flags.push_str(&t.flag_sets_for(&split.rest.body));
    let body = t.wrap_bindings(&owned, body);

    // What the arm has to call the whole value: the name the catch-all bound,
    // and — where the match consumed the subject and the body reads it — the
    // subject's own name, which the arm builds again from what it was handed.
    let mut names: Vec<String> = split.binds.iter().cloned().collect();
    if consuming {
        if let Some(name) = named_subject.filter(|_| mentions_subject(&split.rest.body, &match_expr.expr)) {
            names.push(name);
        }
    }
    let mut written = super::enum_match_over(scrutinee, match_expr, &split.named, t, position);
    let mut arms = String::new();
    for variant in &rest {
        // A borrowing match leaves the enum whole, so the subject *is* the
        // value; a consuming one has only the payload, and the value is that
        // payload back under the variant this arm matched.
        let whole = if consuming {
            format!("new {}('{}', v)", class, variant)
        } else {
            scrutinee.to_string()
        };
        let mut prelude = flags.clone();
        for name in &names {
            prelude.push_str(&format!("const {} = {};\n", name, whole));
        }
        let head = if consuming && !names.is_empty() {
            format!("  {}: (v) => ", variant)
        } else {
            format!("  {}: () => ", variant)
        };
        arms.push_str(&format!(
            "{}{{\n{}  }},\n",
            head,
            indent(&indent(&format!("{}{}", prelude, body)))
        ));
    }
    // The expanded arms go where the runtime match's own arms end.
    let close = written
        .rfind("})")
        .expect("the runtime match is written with a closing brace");
    written.insert_str(close, &arms);
    written
}

/// The TypeScript name of the subject, where the subject is a plain name.
///
/// A subject that is not one cannot be written twice — Rust evaluates it once —
/// and it is also a value no arm's body can name, because Rust has nothing to
/// call it either.
fn subject_name(subject: &syn::Expr) -> Option<String> {
    let syn::Expr::Path(path) = subject else {
        return None;
    };
    let ident = path.path.get_ident()?;
    Some(crate::name_map::escape_reserved(&crate::name_map::to_camel_case(
        &ident.to_string(),
    )))
}

/// Does this arm's body read the subject itself?
///
/// A `_` arm moves nothing, so Rust lets its body use the value the match was
/// given — `_ => MutationError::RetrievalError(err)`. The emitted arm has to
/// have something to call `err`, and only an arm whose body says so needs it.
fn mentions_subject(body: &syn::Expr, subject: &syn::Expr) -> bool {
    let syn::Expr::Path(path) = subject else {
        return false;
    };
    let Some(ident) = path.path.get_ident() else {
        return false;
    };
    names_ident(&quote::ToTokens::to_token_stream(body), ident)
}

/// Is this identifier written anywhere in these tokens?
///
/// A name is a name whatever expression holds it, and the tokens are where
/// every one of them is. It answers yes for a field or method of the same
/// spelling too, which costs a binding the arm does not read and never costs
/// the arm a value it does.
fn names_ident(tokens: &proc_macro2::TokenStream, wanted: &proc_macro2::Ident) -> bool {
    tokens.clone().into_iter().any(|tree| match tree {
        proc_macro2::TokenTree::Ident(ident) => ident == *wanted,
        proc_macro2::TokenTree::Group(group) => names_ident(&group.stream(), wanted),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

    fn built(src: &str) -> Fixture {
        Fixture::build(&[("lib.rs", src)])
    }

    /// `other => other` on a borrowed enum: the arm is written once per variant
    /// the source left to it, and each one hands the subject back.
    #[test]
    fn a_named_catch_all_stands_for_every_variant_left() {
        let mut f = built(
            "pub enum Order { Less, Equal, Greater }\n\
             pub fn pick(o: Order) -> Order {\n\
               match o { Order::Equal => Order::Less, other => other }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "pick");
        assert!(ts.contains("Less: () => {"), "{}", ts);
        assert!(ts.contains("Greater: () => {"), "{}", ts);
        assert_eq!(ts.matches("const other = o;").count(), 2, "{}", ts);
    }

    /// The `_` arm of a consuming match reads the subject the arms above it did
    /// not take. `intoMatch` has moved it, so the arm builds the same value
    /// again out of the variant it matched and the payload it was handed.
    #[test]
    fn a_wildcard_arm_of_a_consuming_match_rebuilds_the_subject() {
        let mut f = built(
            "pub struct Inner;\n\
             pub enum Wrapped { One(Inner), Two }\n\
             pub enum Outer { Held(Inner), Whole(Wrapped) }\n\
             pub fn lift(w: Wrapped) -> Outer {\n\
               match w { Wrapped::One(i) => Outer::Held(i), _ => Outer::Whole(w) }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "lift");
        assert!(ts.contains("w.intoMatch({"), "{}", ts);
        assert!(ts.contains("Two: (v) => {"), "{}", ts);
        assert!(ts.contains("const w = new Wrapped('Two', v);"), "{}", ts);
        assert!(ts.contains("new Outer('Whole', { _0: w })"), "{}", ts);
    }

    /// A `_` arm whose body never names the subject needs no payload at all.
    #[test]
    fn a_wildcard_arm_that_reads_nothing_takes_no_payload() {
        let mut f = built(
            "pub enum Step { A, B, C, D }\n\
             pub fn rank(s: &Step) -> u32 {\n\
               match s { Step::A => 1, Step::B => 2, _ => 0 }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "rank");
        assert!(ts.contains("C: () => {"), "{}", ts);
        assert!(ts.contains("D: () => {"), "{}", ts);
        assert_eq!(ts.matches("return 0;").count(), 2, "{}", ts);
    }

    /// An arm written after the catch-all can never run, and says so.
    #[test]
    fn an_arm_after_the_catch_all_is_reported() {
        let mut f = built(
            "pub enum Step { A, B }\n\
             pub fn rank(s: &Step) -> u32 {\n\
               match s { Step::A => 1, _ => 0, Step::B => 2 }\n\
             }",
        );
        let _ = f.translated_method("lib.rs", "rank");
        assert!(
            f.messages().iter().any(|m| m.contains("never run")),
            "{:?}",
            f.messages()
        );
    }

    /// A subject the engine cannot type has no variant list to write the arm
    /// against, and that is said rather than passed over.
    #[test]
    fn a_catch_all_over_an_untyped_subject_is_reported() {
        let mut f = built(
            "pub fn rank<T>(s: &T) -> u32 {\n\
               match s { Step::A => 1, _ => 0 }\n\
             }",
        );
        let _ = f.translated_method("lib.rs", "rank");
        assert!(
            f.messages().iter().any(|m| m.contains("names no variant")),
            "{:?}",
            f.messages()
        );
    }

    /// A match whose arms all name a variant is still the runtime's own match.
    #[test]
    fn a_match_with_no_catch_all_is_left_alone() {
        let mut f = built(
            "pub enum Step { A, B }\n\
             pub fn rank(s: &Step) -> u32 {\n\
               match s { Step::A => 1, Step::B => 2 }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "rank");
        assert!(ts.starts_with("return s.match({"), "{}", ts);
    }

    /// A catch-all that stands for nothing — every variant already named — is
    /// left out, because Rust cannot reach it either.
    #[test]
    fn a_catch_all_with_nothing_left_to_cover_is_dropped() {
        let mut f = built(
            "pub enum Step { A, B }\n\
             pub fn rank(s: &Step) -> u32 {\n\
               match s { Step::A => 1, Step::B => 2, _ => 0 }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "rank");
        assert!(!ts.contains("return 0;"), "{}", ts);
    }
}
