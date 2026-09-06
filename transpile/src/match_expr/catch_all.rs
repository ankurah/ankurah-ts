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
use super::fallback::{Fallback, Pieces, mentions_subject, subject_name};

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
) -> Result<Remainder, String> {
    let ty = t
        .borrowed_scrutinee_type(&match_expr.expr)
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
    // A named arm covers its variant only when it matches EVERY value of it.
    // `Ex::Literal(Lit::I(n))` names `Literal` and matches one shape of it, so
    // an `Ex::Literal(Lit::S(..))` still has to reach the catch-all; counting
    // the arm by its variant name alone deleted the catch-all and left that
    // value with no arm at all (ankql's `conversion.rs`).
    let named: Vec<String> = split
        .named
        .iter()
        .filter(|arm| arm.guard.is_none() && covers_its_variant(&arm.pat))
        .flat_map(|arm| super::variants_of(&arm.pat))
        .collect();
    let rest: Vec<(String, bool)> = variants
        .iter()
        .filter(|v| !named.contains(&v.name))
        .map(|v| (v.name.clone(), !v.fields.is_empty()))
        .collect();
    // A variant an arm names without covering is written twice if it is left
    // in: once by that arm, once by the expanded catch-all — and the runtime's
    // match has one key per variant.
    let contested: Vec<String> = split
        .named
        .iter()
        .flat_map(|arm| super::variants_of(&arm.pat))
        .filter(|name| rest.iter().any(|(variant, _)| variant == name))
        .collect();
    Ok(Remainder { class: def.name.clone(), rest, contested })
}

/// The variants an arm of this match names without covering, where a catch-all
/// stands below it.
///
/// Asked before the match is written, because the answer decides which FORM it
/// takes: the runtime's `.match({..})` has one arm per variant and nowhere to
/// fall through to, so an arm that tests inside its variant and a catch-all
/// below it cannot both be written there. Nothing is reported here — asking is
/// not translating — so the caller's own diagnostics are the only ones.
pub(super) fn contested(match_expr: &syn::ExprMatch, split: &Split<'_>, t: &BodyTranslator) -> Vec<String> {
    let mark = t.mark();
    let answer = remaining(match_expr, split, t).map(|r| r.contested).unwrap_or_default();
    t.rewind(mark);
    answer
}

/// What the catch-all still has to stand for.
struct Remainder {
    /// The class the port writes the enum as.
    class: String,
    /// The variants no arm named, plus the ones an arm named without covering,
    /// each with whether it carries a payload at all — a unit variant has
    /// nothing for an arm to own.
    rest: Vec<(String, bool)>,
    /// The variants an arm names but does not cover, which is why they are in
    /// `rest` at all.
    contested: Vec<String>,
}

/// Does this arm match every value of the variant it names?
///
/// Only then does it stand for the whole variant. A sub-pattern that asks a
/// question of the payload — a literal, a nested variant, a range — leaves the
/// values it does not match to whatever comes after.
pub(super) fn covers_its_variant(pat: &syn::Pat) -> bool {
    match pat {
        syn::Pat::TupleStruct(ts) => ts.elems.iter().all(BodyTranslator::is_irrefutable),
        syn::Pat::Struct(st) => st.fields.iter().all(|f| BodyTranslator::is_irrefutable(&f.pat)),
        syn::Pat::Path(_) => true,
        syn::Pat::Or(or) => or.cases.iter().all(covers_its_variant),
        syn::Pat::Reference(r) => covers_its_variant(&r.pat),
        syn::Pat::Paren(p) => covers_its_variant(&p.pat),
        // A name with a sub-pattern (`x @ Some(_)`) asks whatever the
        // sub-pattern asks.
        syn::Pat::Ident(ident) => ident.subpat.as_ref().map(|(_, p)| covers_its_variant(p)).unwrap_or(true),
        _ => false,
    }
}

/// The whole match, with one arm per variant the source left to the catch-all.
pub(super) fn lower(
    scrutinee: &str,
    match_expr: &syn::ExprMatch,
    split: &Split<'_>,
    t: &BodyTranslator,
    position: super::Position,
) -> String {
    let Remainder { class, rest, contested } = match remaining(match_expr, split, t) {
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
            let unwritable = super::chain::Fallthrough::Unwritable(format!(
                "the arm that matches anything cannot be written here because {}",
                why
            ));
            return super::enum_match_over(scrutinee, match_expr, &split.named, t, position, &unwritable);
        }
    };
    if rest.is_empty() {
        // Every variant is already named, so Rust's arm is unreachable and
        // rustc says as much. Writing it would be writing a branch that cannot
        // run; leaving it out changes nothing.
        return super::enum_match_over(
            scrutinee,
            match_expr,
            &split.named,
            t,
            position,
            &super::chain::Fallthrough::Exhaustive,
        );
    }
    let takes = t.match_takes(match_expr);
    let consuming = takes == crate::ownership::scrutinee::Takes::Payload;

    // A variant an arm names WITHOUT covering belongs to both: to the arm, for
    // the values its inner pattern matches, and to the catch-all for the rest.
    // The chain inside that variant's key is what holds both, so the expansion
    // leaves it alone rather than writing a second key for it.
    let mut rest = rest;
    rest.retain(|(name, _)| !contested.contains(name));

    let scrutinee_ty = t.borrowed_scrutinee_type(&match_expr.expr);
    // The arms read the subject as well as matching on it, and Rust evaluates
    // it once, so a subject that is not already a name is read into one first.
    let named_subject = subject_name(&match_expr.expr);
    let scrutinee = match (&split.binds, &named_subject) {
        (Some(_), None) if !consuming => t.hoist_name(scrutinee.to_string()),
        _ => scrutinee.to_string(),
    };
    let scrutinee = scrutinee.as_str();

    // What the arm calls the whole value: ONE name, either the one the
    // catch-all bound or — for a `_` arm under `intoMatch` whose body reads the
    // subject — the subject's own. Adding both wrote `const e` twice for
    // `match e { .., e => e }`, which no JavaScript engine will load.
    let bound: Option<String> = match (&split.binds, consuming) {
        (Some(local), _) => Some(local.clone()),
        (None, true) => named_subject
            .clone()
            .filter(|_| mentions_subject(&split.rest.body, &match_expr.expr)),
        (None, false) => None,
    };
    // A borrowing match whose catch-all binds the subject's OWN name is naming
    // the value that is already there: `const e = e;` reads the name it is
    // declaring, which is its temporal dead zone.
    let declares = match (&bound, &named_subject) {
        (Some(name), Some(subject)) => consuming || name != subject,
        (Some(_), None) => true,
        (None, _) => false,
    };

    // Does the match hand a value back at all? Asked the same way
    // `enum_match_over` asks it; asking is not translating, so what the
    // resolution cannot say is not reported here.
    let produces = {
        let mark = t.mark();
        let whole = syn::Expr::Match(match_expr.clone());
        let answer = !matches!(t.resolve_expr_type(&whole), Ok(crate::ty::Ty::Unit));
        t.rewind(mark);
        answer
    };
    let _bindings = t.enter_pattern(&split.rest.pat, scrutinee_ty.as_ref());
    // The name the catch-all binds owns the value from here — Rust moved it in
    // — so an arm that only reads it is what releases it, and one that hands it
    // on sets its flag instead. A BORROWED subject was moved by nothing, and
    // claiming its binding made the arm release a value the caller still owns.
    // `flag_set_for_subject` is the same question asked from the other side, so
    // the two halves ask it once.
    let subject_flag = if consuming { String::new() } else { t.flag_set_for_subject(&match_expr.expr) };
    let subject_is_owned = consuming || !subject_flag.is_empty();
    let owned = match &bound {
        Some(local) if declares && subject_is_owned => t.claim_bindings(
            std::slice::from_ref(local),
            std::slice::from_ref(&syn::Stmt::Expr(split.rest.body.as_ref().clone(), None)),
        ),
        _ => Vec::new(),
    };
    // K2: where the match hands a value back, the body is written for the
    // position that wants one.
    let ((body, value), lifted) =
        t.with_own_hoists(|| super::arms::body_of_an_arm(&split.rest.body, produces, t));
    let body = body.trim_end().to_string();
    drop(_bindings);

    let mut flags = if declares { subject_flag } else { String::new() };
    flags.push_str(&t.flag_sets_for(&split.rest.body));
    let is_async = crate::control_flow::awaiting::awaits(&split.rest.body);
    // A consuming arm owns the whole payload from the moment it is called:
    // `intoMatch` releases nothing of its own on any path out. An arm that
    // rebuilds the value owns it through the value it built; one that does not
    // rebuild it owns the payload directly and says so here.
    let owes_payload = consuming && !declares;

    // One body, however many variants are left to it. The expansion writes the
    // catch-all's body once per remaining variant, and `core/src/value/index.ts`
    // had fifteen copies of one six-line body; a local closure is one copy the
    // arms call. Not where the body awaits — the arms would each have to await
    // it, and the match with them — and not for a single arm, which would be
    // one indirection for nothing.
    // Two lines or fewer is shorter written out than called: the closure and
    // its declaration cost more than the copies save.
    let worth_hoisting = rest.len() > 1 && !is_async && body.lines().count() > 2;
    let hoisted_body = worth_hoisting.then(|| {
        let inner = t.wrap_bindings(&owned, crate::ownership::hoisted(&super::arm_statements(&body, produces, value), &lifted));
        let param = match (&bound, declares) {
            (Some(name), true) => name.clone(),
            _ => String::new(),
        };
        t.hoist_name(format!("({}) => {{\n{}}}", param, indent(&inner)))
    });

    let fallback = Fallback {
        class: &class,
        consuming,
        scrutinee,
        bound: bound.as_deref(),
        declares,
        flags: &flags,
        body: &body,
        owned: &owned,
        lifted: &lifted,
        produces,
        value,
        is_async,
        owes_payload,
        hoisted: hoisted_body.as_deref(),
        rest_body: &split.rest.body,
    };
    let mut written = super::enum_match_over(
        scrutinee,
        match_expr,
        &split.named,
        t,
        position,
        &super::chain::Fallthrough::CatchAll(&fallback),
    );
    let mut arms = String::new();
    for (variant, has_payload) in &rest {
        let Pieces { bindings, release, takes_payload, whole } = fallback.pieces(variant, *has_payload, "v");
        if let Some(rest_body) = &hoisted_body {
            // The arm is the call. What it owns it settles here — the payload
            // no name took — and the flags it owes stand before the call.
            let call = match (&bound, declares) {
                (Some(_), true) => format!("{}({})", rest_body, whole),
                _ => format!("{}()", rest_body),
            };
            let inner = if release.is_empty() {
                format!("return {};\n", call)
            } else {
                format!("try {{\n  return {};\n}} finally {{\n{}}}\n", call, indent(&release))
            };
            let head = if takes_payload {
                format!("  {}: (v) => ", variant)
            } else {
                format!("  {}: () => ", variant)
            };
            arms.push_str(&format!(
                "{}{{\n{}  }},\n",
                head,
                indent(&indent(&format!("{}{}", flags, inner)))
            ));
            continue;
        }
        arms.push_str(&super::render_arm(
            super::ArmParts {
                variant,
                bindings,
                param: takes_payload.then(|| "v".to_string()),
                body: &body,
                owned: &owned,
                lifted: &lifted,
                produces,
                value,
                is_async,
                release_rest: release,
                // A chain link and a catch-all are always written as a
                // block, so the bare-expression cast never applies to them.
                tuple: false,
            },
            t,
        ));
    }
    // The expanded arms go where the runtime match's own arms end.
    let close = written
        .rfind("})")
        .expect("the runtime match is written with a closing brace");
    written.insert_str(close, &arms);
    if is_async && !written.starts_with("await ") {
        written = format!("await ({})", written);
    }
    written
}

#[cfg(test)]
#[path = "catch_all_tests.rs"]
mod tests;
