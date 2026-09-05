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
fn covers_its_variant(pat: &syn::Pat) -> bool {
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

    let mut rest = rest;
    if !contested.is_empty() {
        // An arm that tests INSIDE a variant and a catch-all below it need one
        // form that can do both: test the payload, and fall through to the next
        // arm when the test fails. A borrowing match is written as the if-chain
        // that can, and never reaches here. This one hands its payload to the
        // arms, which needs `intoMatch` — one arm per variant, with nowhere to
        // fall through to — so the values the testing arm does not match have
        // no arm of their own, and that is said rather than passed over.
        t.report_match_gap(
            match_expr,
            format!(
                "an arm tests inside `{}` and a later arm matches anything, which needs a test \
                 that can fall through — and this match hands its payload to the arms, which \
                 needs `intoMatch`, whose one arm per variant has nowhere to fall through to; \
                 the arm that tests runs for every `{}`",
                contested.join("`, `"),
                contested.join("`, `"),
            ),
        );
        rest.retain(|(name, _)| !contested.contains(name));
        if rest.is_empty() {
            return super::enum_match_over(scrutinee, match_expr, &split.named, t, position);
        }
    }

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
    let (body, lifted) = t.with_own_hoists(|| t.statements(&split.rest.body));
    let body = body.trim_end().to_string();
    drop(_bindings);

    let mut flags = if declares { subject_flag } else { String::new() };
    flags.push_str(&t.flag_sets_for(&split.rest.body));
    let is_async = crate::control_flow::awaiting::awaits(&split.rest.body);
    // Does the match hand a value back at all? Asked the same way
    // `enum_match_over` asks it, and asking is not translating, so what the
    // resolution cannot say is not reported here.
    let produces = {
        let mark = t.mark();
        let whole = syn::Expr::Match(match_expr.clone());
        let answer = !matches!(t.resolve_expr_type(&whole), Ok(crate::ty::Ty::Unit));
        t.rewind(mark);
        answer
    };
    // A consuming arm owns the whole payload from the moment it is called:
    // `intoMatch` releases nothing of its own on any path out. An arm that
    // rebuilds the value owns it through the value it built; one that does not
    // rebuild it owns the payload directly and says so here.
    let release_rest = if consuming && !declares { "dropUnbound(v, []);\n".to_string() } else { String::new() };

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
        let inner = t.wrap_bindings(&owned, crate::ownership::hoisted(&super::arm_statements(&body, produces), &lifted));
        let param = match (&bound, declares) {
            (Some(name), true) => name.clone(),
            _ => String::new(),
        };
        t.hoist_name(format!("({}) => {{\n{}}}", param, indent(&inner)))
    });

    let mut written = super::enum_match_over(scrutinee, match_expr, &split.named, t, position);
    let mut arms = String::new();
    for (variant, has_payload) in &rest {
        // A borrowing match leaves the enum whole, so the subject *is* the
        // value; a consuming one has only the payload, and the value is that
        // payload back under the variant this arm matched.
        let whole = if consuming {
            format!("new {}('{}', v)", class, variant)
        } else {
            scrutinee.to_string()
        };
        let bindings = match (&bound, declares) {
            (Some(name), true) => format!("{}const {} = {};\n", flags, name, whole),
            _ => flags.clone(),
        };
        // A unit variant's payload is empty, so there is nothing in it to own.
        let release = if *has_payload { release_rest.clone() } else { String::new() };
        let takes_payload = consuming && (declares || !release.is_empty());
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
                is_async,
                release_rest: release,
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

    /// A `_` arm over a BORROWED enum whose variants carry nothing needs no
    /// payload: the enum stays whole and there is nothing in it to own.
    #[test]
    fn a_wildcard_arm_over_a_borrowed_payload_free_enum_takes_no_payload() {
        let mut f = built(
            "pub enum Step { A, B, C, D }\n\
             pub fn rank(s: &Step) -> u32 {\n\
               match s { Step::A => 1, Step::B => 2, _ => 0 }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "rank");
        assert!(ts.contains("C: () => 0,"), "{}", ts);
        assert!(ts.contains("D: () => 0,"), "{}", ts);
    }

    /// PREMISE CHANGED 2026-09-04: the test this replaces asserted that a `_`
    /// arm which reads nothing takes no payload, full stop. Under `intoMatch`
    /// that is a leak — the payload is handed over and nobody receives it — so
    /// the rule is now that a CONSUMING arm always takes the payload and owns
    /// all of it, and only a borrowing arm over a payload-free enum can decline
    /// it.
    #[test]
    fn a_consuming_wildcard_arm_releases_the_payload_it_reads_nothing_of() {
        let mut f = built(
            "pub struct Inner;\n\
             pub enum Step { Taken(Inner), Rest(Inner) }\n\
             pub fn rank(s: Step) -> u32 {\n\
               match s { Step::Taken(i) => 1, _ => 0 }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "rank");
        assert!(ts.contains("s.intoMatch({"), "{}", ts);
        assert!(ts.contains("Rest: (v) => {"), "{}", ts);
        assert!(ts.contains("dropUnbound(v, []);"), "{}", ts);
    }

    /// A named arm that ignores its payload owns it just the same: `intoMatch`
    /// hands the whole thing over and releases nothing of its own.
    #[test]
    fn a_named_arm_that_ignores_its_payload_releases_it() {
        let mut f = built(
            "pub struct Inner;\n\
             pub enum Step { Taken(Inner), Rest(Inner) }\n\
             pub fn rank(s: Step) -> u32 {\n\
               match s { Step::Taken(_) => 1, Step::Rest(i) => 2 }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "rank");
        assert!(ts.contains("Taken: (v) => {"), "{}", ts);
        assert!(ts.contains("dropUnbound(v, []);"), "{}", ts);
    }

    /// A catch-all that binds the scrutinee's own name binds the value once.
    /// `match e { E::Taken(t) => .., e => e }` wrote `const e` twice, which no
    /// JavaScript engine will load.
    #[test]
    fn a_catch_all_that_shadows_the_subject_declares_it_once() {
        let mut f = built(
            "pub struct Inner;\n\
             pub enum E { Taken(Inner), Rest(Inner) }\n\
             pub fn keep(e: E) -> E {\n\
               match e { E::Taken(t) => E::Rest(t), e => e }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "keep");
        assert_eq!(ts.matches("const e = new E('Rest', v);").count(), 1, "{}", ts);
    }

    /// A borrowing catch-all that binds the scrutinee's own name declares
    /// nothing: `const e = e;` reads the name it is declaring.
    #[test]
    fn a_borrowing_catch_all_that_shadows_the_subject_declares_nothing() {
        let mut f = built(
            "pub enum E { A, B, C }\n\
             pub fn count(e: &E) -> u32 {\n\
               match e { E::A => 1, e => 0 }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "count");
        assert!(!ts.contains("const e = e;"), "{}", ts);
    }

    /// A catch-all that binds a BORROWED subject does not release it: the
    /// caller still owns it, and the arm dropping it was a double drop.
    #[test]
    fn a_catch_all_binding_a_borrowed_subject_releases_nothing() {
        let mut f = built(
            "pub struct Inner;\n\
             pub enum E { A(Inner), B(Inner), C(Inner) }\n\
             pub struct Holder { pub choice: E }\n\
             pub fn pick(h: &Holder) -> u32 {\n\
               match &h.choice { E::A(_) => 1, other => 0 }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "pick");
        assert!(!ts.contains("other.drop()"), "{}", ts);
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

    /// PREMISE CHANGED 2026-09-04: "every variant is already named" used to be
    /// read off the variant NAMES alone, so an arm that named a variant and
    /// tested inside it counted as covering the whole of it and the catch-all
    /// was deleted — leaving the values that arm does not match with no arm at
    /// all. An arm covers its variant only when it matches every value of it,
    /// which is the premise the test below now states, and the refutable case
    /// is the test after it.
    ///
    /// A catch-all that stands for nothing — every variant already covered — is
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

    /// An arm that tests INSIDE its variant does not cover the variant, so the
    /// catch-all still stands for the values it does not match. The runtime's
    /// match cannot express "test the payload, and fall through if it fails",
    /// so a borrowing match goes to the if-chain, which can.
    #[test]
    fn an_arm_that_tests_inside_its_variant_does_not_delete_the_catch_all() {
        let mut f = built(
            "pub enum Lit { S, I }\n\
             pub enum Ex { Literal(Lit), Path }\n\
             pub fn rank(e: &Ex) -> u32 {\n\
               match e { Ex::Literal(Lit::I) => 7, Ex::Path => 1, _ => 99 }\n\
             }",
        );
        let ts = f.translated_method("lib.rs", "rank");
        assert!(!ts.contains(".match({"), "{}", ts);
        assert!(ts.contains("99"), "{}", ts);
        assert!(ts.contains("is('Literal')"), "{}", ts);
        assert!(ts.contains("is('I')"), "{}", ts);
    }

    /// The consuming form has no such rewrite — the if-chain reads the payload
    /// without marking the enum moved — so it says so rather than running the
    /// testing arm for every value of the variant in silence.
    #[test]
    fn a_consuming_match_that_tests_inside_a_variant_is_reported() {
        let mut f = built(
            "pub struct Inner;\n\
             pub enum Lit { S(Inner), I(Inner) }\n\
             pub enum Ex { Literal(Lit), Path(Inner) }\n\
             pub fn rank(e: Ex) -> u32 {\n\
               match e { Ex::Literal(Lit::I(i)) => 7, _ => 99 }\n\
             }",
        );
        let _ = f.translated_method("lib.rs", "rank");
        assert!(
            f.messages().iter().any(|m| m.contains("nowhere to fall through to")),
            "{:?}",
            f.messages()
        );
    }
}
