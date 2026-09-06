//! Match expression translation — Rust match → TS patterns
//!
//! Handles Option match (Some/None → null checks), Result match (Ok/Err → try/catch),
//! and enum match (variants → .match({}) pattern).

mod arms;
mod catch_all;
mod fallback;
mod chain;
mod option_chain;

/// What a consuming arm owes the payload it was handed.
mod owing;
mod payload;
mod rendering;
mod taking;
mod result_arms;

mod value_match;
use value_match::{subject_of_bound, translate_value_match};

use crate::body::{translate_pat, indent, BodyTranslator};
use crate::control_flow::sentinel::{jumps_in, jumps_out, leaves_the_function};
use arms::{arm_statements, payload_of, render_arm, ArmParts};

/// Translate a match expression in return position (adds return to each arm)
pub fn translate_match_returning(match_expr: &syn::ExprMatch, t: &BodyTranslator) -> String {
    let written = returning(match_expr, t);
    // Every strategy writes STATEMENTS here: the keyed form is the one
    // expression among them and it is wrapped in a `return` of its own. Set
    // after the strategies run, because an arm holding a match of its own
    // would otherwise be the last to have its say.
    t.last_match_wrote_statements.set(true);
    written
}

fn returning(match_expr: &syn::ExprMatch, t: &BodyTranslator) -> String {
    let scrutinee = scrutinee_of(match_expr, t);
    if let Some(written) = guarded(&scrutinee, match_expr, t, Position::Returning) {
        return written;
    }
    if is_option_match_typed(match_expr, t) {
        return option_chain::translate(&scrutinee, match_expr, t, Position::Returning);
    }
    if is_result_match(&match_expr.arms) {
        return translate_result_match(&scrutinee, match_expr, t, Position::Returning);
    }
    // An ordering is a number, so a `match` on one is a chain of comparisons.
    // The runtime's `.match({..})` dispatches on a variant name, and a number
    // has none.
    if t.is_ordering_value(&match_expr.expr) {
        return translate_value_match(&scrutinee, match_expr, t, Position::Returning);
    }
    if looks_like_enum_match(&match_expr.arms) {
        if let Some(written) = leaves_the_loop(&scrutinee, match_expr, t, Position::Returning) {
            return written;
        }
        if let Some(written) = tests_inside_a_variant(&scrutinee, match_expr, t, Position::Returning) {
            return written;
        }
        return format!(
            "return {};",
            translate_enum_match(&scrutinee, match_expr, t, Position::Returning)
        );
    }
    translate_value_match(&scrutinee, match_expr, t, Position::Returning)
}

/// The subject, written for what the arms are about to do to it.
///
/// A consuming match takes the subject apart and leaves it moved, so where the
/// subject is a field of something the block still owns, the field has to come
/// *out* of its struct: `holder.choice.intoMatch(..)` hands the payload to an
/// arm and then lets `holder`'s own cascade release the enum the arm has
/// already taken, which the runtime reports as a use after move.
fn scrutinee_of(match_expr: &syn::ExprMatch, t: &BodyTranslator) -> String {
    match t.match_takes(match_expr) {
        crate::ownership::scrutinee::Takes::Payload => t.moved_value(&match_expr.expr),
        crate::ownership::scrutinee::Takes::Nothing => t.expr(&match_expr.expr),
    }
}

/// Whether each arm produces the enclosing function's value or just runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Position {
    Statement,
    Returning,
}


/// Translate a match expression.
///
/// Records which FORM it wrote in `t.last_match_wrote_statements`: only the
/// runtime's keyed `.match({..})` is one expression, and every other strategy
/// here is an if-chain. The position that asked reads it straight afterwards
/// (`control_flow::form::writes_statements`), which is what replaced reading
/// the punctuation of the text back.
pub fn translate_match(match_expr: &syn::ExprMatch, t: &BodyTranslator) -> String {
    let (written, statements) = statement_position(match_expr, t);
    // Set after the strategies run: an arm holding a match of its own would
    // otherwise be the last to have its say about this one.
    t.last_match_wrote_statements.set(statements);
    written
}

fn statement_position(match_expr: &syn::ExprMatch, t: &BodyTranslator) -> (String, bool) {
    let scrutinee = scrutinee_of(match_expr, t);
    if let Some(written) = guarded(&scrutinee, match_expr, t, Position::Statement) {
        return (written, true);
    }
    if is_option_match_typed(match_expr, t) {
        return (option_chain::translate(&scrutinee, match_expr, t, Position::Statement), true);
    }
    if is_result_match(&match_expr.arms) {
        return (translate_result_match(&scrutinee, match_expr, t, Position::Statement), true);
    }
    // An ordering is a number, so a `match` on one is a chain of comparisons.
    // The runtime's `.match({..})` dispatches on a variant name, and a number
    // has none.
    if t.is_ordering_value(&match_expr.expr) {
        return (translate_value_match(&scrutinee, match_expr, t, Position::Statement), true);
    }
    if looks_like_enum_match(&match_expr.arms) {
        if let Some(written) = leaves_the_loop(&scrutinee, match_expr, t, Position::Statement) {
            return (written, true);
        }
        if let Some(written) = tests_inside_a_variant(&scrutinee, match_expr, t, Position::Statement) {
            return (written, true);
        }
        // The one form that is an EXPRESSION: `subject.match({ A: () => .. })`.
        return (translate_enum_match(&scrutinee, match_expr, t, Position::Statement), false);
    }

    (translate_value_match(&scrutinee, match_expr, t, Position::Statement), true)
}

/// A match with an arm that tests inside its variant and a catch-all below it,
/// written as the if-chain that can do both.
///
/// The runtime's `.match({..})` dispatches on the variant name and has one arm
/// per variant, so it can neither test inside a payload nor fall through to the
/// next arm when that test fails. `Ex::Literal(Lit::I(n)) => .., _ => 99` needs
/// both. The if-chain over `pattern_test` is the form that has them.
///
/// A match that hands its payload to the arms has no such form — the if-chain
/// reads the payload without marking the enum moved — so that one goes to the
/// expansion, which reports it.
fn tests_inside_a_variant(
    scrutinee: &str,
    match_expr: &syn::ExprMatch,
    t: &BodyTranslator,
    position: Position,
) -> Option<String> {
    let split = catch_all::split(match_expr)?;
    if catch_all::contested(match_expr, &split, t).is_empty() {
        return None;
    }
    if t.match_takes(match_expr) == crate::ownership::scrutinee::Takes::Payload {
        return None;
    }
    Some(translate_value_match(scrutinee, match_expr, t, position))
}

/// A match whose arm leaves the loop around it, written as the if-chain that
/// keeps the arm inside the loop.
///
/// The runtime's `match` runs each arm as a function, and `break` and
/// `continue` cannot leave one: `Object: (v) => continue` does not even parse.
/// The if-chain writes each arm as a block of the enclosing function, where
/// both keywords mean what Rust meant by them.
///
/// A match that hands its payload to the arms has no such form — the if-chain
/// reads the payload without marking the enum moved, and the enum's owner would
/// release what the arm took — so that is reported rather than rewritten.
fn leaves_the_loop(
    scrutinee: &str,
    match_expr: &syn::ExprMatch,
    t: &BodyTranslator,
    position: Position,
) -> Option<String> {
    let jumps = match_expr.arms.iter().any(|arm| jumps_out(&arm.body));
    // An arm of `match` or `intoMatch` is an arrow function either way, so a
    // `?` or a `return` written in one leaves the ARM. Where the match's own
    // value is what the function returns, the arm's `return` is the function's
    // and that is right; where the value is discarded, so is the error.
    let exits = match_expr
        .arms
        .iter()
        .any(|arm| leaves_the_function(&arm.body));
    if !jumps && !exits {
        return None;
    }
    if exits && !jumps {
        // In return position the arm's `return` IS the function's return: the
        // match's value is what the function answers with, so the exit needs no
        // carrying.
        if position == Position::Returning {
            return None;
        }
        // Already inside a lifted body that carries exits out, AND this match's
        // value is what that body hands back: the arm's sentinel travels
        // through it into the enclosing arrow, where the test that performs
        // the return already stands, so a second test here would read a value
        // nobody put there.
        //
        // In STATEMENT position it travels nowhere: the value is discarded on
        // the spot. ankql's `generate_expr_sql` writes `match expr { .. }`
        // inside a `for` inside an arm, every branch of it `return Err(..)`,
        // and the sentinel each one produced was dropped where it stood — the
        // error never left the function, and the `Result.Err` and its payload
        // leaked. So the test is written here too, and it hands the sentinel
        // ON rather than unwrapping it, because a `return` here leaves the arm
        // and not the function.
        if t.jump_as_value.get() && position != Position::Statement {
            return None;
        }
    }
    if jumps && t.match_takes(match_expr) != crate::ownership::scrutinee::Takes::Payload {
        // A borrowing match can be written as the if-chain, where each arm is
        // a block of the enclosing function and both keywords mean what Rust
        // meant by them — and a `?` written in such a block is a real return.
        return Some(translate_value_match(scrutinee, match_expr, t, position));
    }
    // The if-chain is not available for a consuming match — it would read the
    // payload without marking the enum moved — so the arms stay functions and
    // the jump is handed back as a value. Each arm settles what it owns first,
    // in its own `finally`, and the caller performs the jump.
    //
    // Only where the match's own value is discarded. Where something wants the
    // value, the sentinel would be standing in the place of that value and
    // there is nowhere to put the test. A statement's value is discarded
    // whatever it is, and a match whose own Rust value is `()` has none.
    if position == Position::Statement || !produces_a_value(match_expr, t) {
        return Some(jump_through_a_value(scrutinee, match_expr, t));
    }
    if jumps {
        t.report_match_gap(
            match_expr,
            "an arm of this `match` leaves the loop around it, the match hands its payload to \
             the arms — which needs `intoMatch`, whose arms are functions a `break` cannot \
             leave — and the match stands where its value is wanted, so there is nowhere to \
             put the test that would perform the jump",
        );
    }
    None
}

/// Does this match hand a value back at all?
///
/// A `match` whose Rust value is `()` is run for what its arms do. Asking is
/// not translating, so what the resolution cannot say is not reported here.
fn produces_a_value(match_expr: &syn::ExprMatch, t: &BodyTranslator) -> bool {
    let mark = t.mark();
    let whole = syn::Expr::Match(match_expr.clone());
    let answer = !matches!(t.resolve_expr_type(&whole), Ok(crate::ty::Ty::Unit));
    t.rewind(mark);
    answer
}

/// A consuming match whose arms jump, written as the value they hand back.
///
/// `Payload: (v) => { .. return break; }` does not parse. The arm returns a
/// sentinel instead, and the statement after the match reads it and performs
/// the jump — after the arm's own `finally` has released whatever it held,
/// which is the order Rust unwinds a `break` out of a scope in.
fn jump_through_a_value(
    scrutinee: &str,
    match_expr: &syn::ExprMatch,
    t: &BodyTranslator,
) -> String {
    let (written, _) = crate::control_flow::sentinel::lifting(t, || {
        translate_enum_match(scrutinee, match_expr, t, Position::Statement)
    });

    let jumping: Vec<&syn::Arm> = match_expr.arms.iter().filter(|arm| jumps_out(&arm.body)).collect();
    let mut kinds: Vec<String> = Vec::new();
    for arm in jumping {
        for jump in jumps_in(&arm.body) {
            if !kinds.contains(&jump) {
                kinds.push(jump);
            }
        }
    }
    let held = t.hoist_name(written);
    let reader = crate::control_flow::sentinel::reader(
        t,
        &held,
        crate::control_flow::sentinel::Handed {
            returns: match_expr
                .arms
                .iter()
                .any(|arm| leaves_the_function(&arm.body)),
            jumps: &kinds,
        },
    );
    reader.trim_end().to_string()
}

/// A match with a guard, written as the if-chain that tries its arms in turn.
///
/// A guard reads the names its own pattern bound, so it cannot be written where
/// the pattern's test is written, and an arm whose guard fails hands the value
/// to the arm BELOW it. The if-chain has both: each arm opens a block, binds
/// its names in it, tests the guard there, and leaves the chain when it runs.
///
/// This is the form for a match the runtime has no `match` of its own for — a
/// number, a string, a tuple. The three that do have one carry their guards
/// themselves and are left alone here: an `Option` match through
/// `option_chain`, a `Result` match through `translate_result_match`, and an
/// enum match through the per-variant arm chain, whose links took a guard of
/// their own in the sixth pass. Before that the enum and `Result` forms
/// reported "the guard is dropped" and ran the arm unconditionally — live at
/// `core/src/node.rs:621`, where an EMPTY event bridge answered the bridge
/// path, and at `core/src/context.rs:187`, where a cached entity with no
/// durable peers answered an error.
fn guarded(
    scrutinee: &str,
    match_expr: &syn::ExprMatch,
    t: &BodyTranslator,
    position: Position,
) -> Option<String> {
    if !match_expr.arms.iter().any(|arm| arm.guard.is_some()) {
        return None;
    }
    // A `Result` match carries its guards itself: each side reads its payload
    // once and tries the arms that name its variant against it.
    if is_result_match(&match_expr.arms) {
        return None;
    }
    // A match that only READS its subject is written here, whatever its subject
    // is: the if-chain reads, which is all a borrow needs, and it has carried
    // guards since before the arm chain existed. Sending a borrowed enum match
    // to the chain instead cost something — an arm whose body is a nested match
    // loses its `return`, which is the open F3 gap about deciding `return` from
    // punctuation — for nothing the if-chain was not already doing.
    if t.match_takes(match_expr) != crate::ownership::scrutinee::Takes::Payload {
        return Some(translate_value_match(scrutinee, match_expr, t, position));
    }
    // A CONSUMING match is the one the if-chain has no form for: nothing here
    // marks the subject moved, so the subject's owner would release what an arm
    // has taken. An enum match goes to the per-variant arm chain, which does
    // both — but only for a guard on an arm that NAMES a variant, because a
    // guarded CATCH-ALL has no key of its own for the chain to hang off.
    let every_guard_names_a_variant = match_expr
        .arms
        .iter()
        .filter(|arm| arm.guard.is_some())
        .all(|arm| arms::cases_of(&arm.pat).iter().all(|case| payload_of(case).is_some()));
    if !is_option_match_typed(match_expr, t)
        && looks_like_enum_match(&match_expr.arms)
        && every_guard_names_a_variant
    {
        return None;
    }
    // R12 rather than a wrong answer — and a hole releases what it was handed.
    // The subject is an expression the source wrote to be consumed here, so it
    // is evaluated (its side effects are the program's) into a name, released,
    // and the throw stands after both.
    let what = "an arm of this `match` has a guard and the match hands its payload to the \
                arms, and the if-chain a guard needs reads the subject without marking it \
                moved; no form of this match is written";
    t.report_match_gap(match_expr, what);
    let throw = format!("{};", crate::body::hole_text(what));
    let held = t.fresh_hoist("_h");
    let Some(release) = t.release_of(&match_expr.expr, &held) else {
        return Some(throw);
    };
    Some(format!("const {} = {};\n{}\n{}", held, scrutinee, release, throw))
}

/// Is this a match on an `Option`?
///
/// The subject's type answers it where the engine has one: `ConnectionState`
/// has a variant literally called `None`, and reading the arm names alone sent
/// its match down the `T | null` path, which wrote `if (this._0 == null)` for a
/// variant test and put an `if` where the `return` wanted a value.
fn is_option_match_typed(match_expr: &syn::ExprMatch, t: &BodyTranslator) -> bool {
    // `resolve_expr_type`, not `scrutinee_type`: the latter reports a subject it
    // cannot type, and the paths below ask the same question again, so asking it
    // here too counted one gap twice.
    if let (Ok(ty), Some(reg)) = (t.resolve_expr_type(&match_expr.expr), t.registry()) {
        if let Some(id) = ty.peel_refs().id() {
            return reg.name_of(id) == "Option";
        }
    }
    // No type for the subject: the arm names are all there is to go on.
    is_option_match(&match_expr.arms)
}

fn is_option_match(arms: &[syn::Arm]) -> bool {
    arms.iter().any(|arm| {
        match &arm.pat {
            syn::Pat::TupleStruct(ts) => {
                let name = ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                name == "Some" || name == "None"
            }
            syn::Pat::Ident(ident) => ident.ident == "None",
            syn::Pat::Path(path) => {
                path.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default() == "None"
            }
            _ => false,
        }
    })
}

pub(crate) fn is_result_match(arms: &[syn::Arm]) -> bool {
    arms.iter().any(|arm| {
        if let syn::Pat::TupleStruct(ts) = &arm.pat {
            let name = ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
            name == "Ok" || name == "Err"
        } else {
            false
        }
    })
}

/// `match r { Ok(v) => .., Err(e) => .. }`.
///
/// A `Result` is a value here, not a thrown error, so the match is the test
/// `isOk()` and one consuming read on the branch that was taken: `unwrap()` or
/// `unwrapErr()`, each of which takes the wrapper and hands back what it held.
/// Binding the name to the `Result` itself gave the arm the wrapper where the
/// payload belonged and lost the `Err` arm entirely.
///
/// A side with SEVERAL arms — `Err(NoDurablePeers) if cached` beside `Err(e)` —
/// cannot read the wrapper twice, so it reads the payload once and tries its
/// arms against that: `result_arms::side` writes it.
fn translate_result_match(
    scrutinee: &str,
    match_expr: &syn::ExprMatch,
    t: &BodyTranslator,
    position: Position,
) -> String {
    let scrutinee_ty = t.borrowed_scrutinee_type(&match_expr.expr);
    let binds: Vec<String> = match_expr
        .arms
        .iter()
        .flat_map(|arm| crate::body::pattern_names(&arm.pat))
        .collect();
    let (subject, declaration) = subject_of_bound(scrutinee, &binds, t);

    // The two sides, written in the order the source names them so that the
    // temporaries each takes are numbered where Rust evaluates them.
    let (ok, err) = result_arms::both_sides(match_expr, &subject, t, position, scrutinee_ty.as_ref());

    let (Some(ok), Some(err)) = (ok, err) else {
        t.report_match_gap(
            match_expr,
            "this `Result` match has no arm for one of the two variants; nothing is emitted \
             for it and no arm runs",
        );
        return format!("{}/* match {} */", declaration, subject);
    };
    format!(
        "{}if ({}.isOk()) {{\n{}}} else {{\n{}}}",
        declaration,
        subject,
        indent(&ok),
        indent(&err)
    )
}

/// The name a one-slot variant pattern binds, where it binds one.
fn payload_binding(pat: &syn::Pat) -> Option<String> {
    let syn::Pat::TupleStruct(ts) = pat else {
        return None;
    };
    match ts.elems.first()? {
        syn::Pat::Ident(_) => Some(translate_pat(ts.elems.first()?)),
        _ => None,
    }
}

/// The variant a tuple-struct pattern names.
fn variant_named(pat: &syn::Pat) -> Option<String> {
    let syn::Pat::TupleStruct(ts) = pat else {
        return None;
    };
    Some(ts.path.segments.last()?.ident.to_string())
}

fn looks_like_enum_match(arms: &[syn::Arm]) -> bool {
    arms.iter().any(|arm| {
        match &arm.pat {
            syn::Pat::TupleStruct(ts) => {
                let name = ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                name != "Some" && name != "None" && name != "Ok" && name != "Err"
                    && name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            }
            syn::Pat::Struct(s) => {
                let name = s.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            }
            syn::Pat::Path(p) => p.path.segments.len() >= 2,
            _ => false,
        }
    })
}

/// `match e { Variant(x) => .. }` as the runtime's own match.
///
/// `intoMatch` is the by-value form: it hands the payload to the arm as the
/// arm's own and leaves the enum moved, so nothing drops it afterwards and the
/// arm releases what it was given. `match` lends the payload and leaves the
/// enum whole, which is what a match on a reference needs. Which one this is
/// comes from the subject's type and the arms' patterns, not from the spelling.
fn translate_enum_match(
    scrutinee: &str,
    match_expr: &syn::ExprMatch,
    t: &BodyTranslator,
    position: Position,
) -> String {
    // An arm naming no variant has no key in the runtime's match. It is written
    // as the test the arms above it amount to, with the catch-all as the else.
    if let Some(split) = catch_all::split(match_expr) {
        let after = catch_all::unreachable_after(match_expr);
        if after > 0 {
            t.report_match_gap(
                match_expr,
                format!(
                    "{} arm(s) stand after the one that matches anything, and Rust tries arms in \
                     order, so they never run",
                    after
                ),
            );
        }
        return catch_all::lower(scrutinee, match_expr, &split, t, position);
    }
    enum_match_over(
        scrutinee,
        match_expr,
        &arms_of(match_expr),
        t,
        position,
        &chain::Fallthrough::Exhaustive,
    )
}

/// Every arm, as the slice the writer below takes.
fn arms_of(match_expr: &syn::ExprMatch) -> Vec<&syn::Arm> {
    match_expr.arms.iter().collect()
}

/// The variant names a pattern stands for: one, or several where it is written
/// with `|`, and none where it names no variant at all.
fn variants_of(pat: &syn::Pat) -> Vec<String> {
    match pat {
        syn::Pat::Or(or) => or.cases.iter().flat_map(variants_of).collect(),
        other => payload_of(other).map(|(v, _)| v).into_iter().collect(),
    }
}
/// The runtime's own match written over exactly these arms.
///
/// One key per variant. Where several arms name one variant — or where one arm
/// names a variant it does not COVER and a catch-all stands below — the key
/// holds the arm CHAIN instead: Rust tries those arms in order against the
/// patterns inside the payload, and `chain::write` is that order made explicit.
fn enum_match_over(
    scrutinee: &str,
    match_expr: &syn::ExprMatch,
    written_arms: &[&syn::Arm],
    t: &BodyTranslator,
    position: Position,
    fall: &chain::Fallthrough<'_>,
) -> String {
    let scrutinee_ty = t.borrowed_scrutinee_type(&match_expr.expr);
    // Does the match hand a value back at all? A `match` whose Rust value is
    // `()` is run for what its arms do, and an arm that returned what the
    // port's own spelling produced gave it one anyway. Asking is not
    // translating, so what this resolution cannot say is not reported here.
    let produces = {
        let mark = t.mark();
        let whole = syn::Expr::Match(match_expr.clone());
        let answer = !matches!(t.resolve_expr_type(&whole), Ok(crate::ty::Ty::Unit));
        t.rewind(mark);
        answer
    };
    let takes = t.match_takes(match_expr);
    let method = match takes {
        crate::ownership::scrutinee::Takes::Payload => "intoMatch",
        crate::ownership::scrutinee::Takes::Nothing => "match",
    };
    // Where an arm hands a jump back as a sentinel, the arms no longer agree
    // about what they answer: one gives the sentinel object and the rest give
    // the match's own value, and `match<R>` takes ONE `R` inferred from all of
    // them — whichever arm TypeScript reaches first. Naming `R` as `any` says
    // that the answer is read by the test the caller writes and by nothing
    // else, and keeps the arms from having to agree.
    let out_ty = if t.jump_as_value.get() { "<any>" } else { "" };

    // A variant several arms name, and a variant one arm names without covering
    // it, are both written as ONE key holding the chain those arms describe.
    let has_catch_all = !matches!(fall, chain::Fallthrough::Exhaustive);
    let contested = chain::contested(written_arms, has_catch_all);
    // The chain's arms share one parameter, so the name has to avoid every name
    // any of their bodies declares — and the catch-all's, which stands as the
    // chain's last `else`. It is chosen before any of them is translated,
    // because the bindings are written out of it.
    let chain_params = chain::parameters(written_arms, &contested, fall);

    // The keys are written in the order the source names them, which is where
    // a chained variant's key stands too — at its FIRST arm. Each arm is
    // translated where the source wrote it, so the temporaries the bodies take
    // are numbered in source order.
    // One `intoMatch` hands back ONE value, and TypeScript types every key's
    // result together: an `async` key beside a plain one makes that type
    // `Promise<T> | T`, which tsc refuses where the match's value is used. Rust's
    // arms all produce the same type, and awaiting a value that is not a promise
    // costs a turn and nothing else — so where any arm awaits, every arm is
    // `async` and the whole match is awaited.
    //
    // A guard is part of its arm: asked of the arm's BODY alone, an awaited
    // guard wrote `await` inside a plain arrow.
    let any_async = written_arms.iter().any(|arm| {
        crate::control_flow::awaiting::awaits(&arm.body)
            || arm
                .guard
                .as_ref()
                .is_some_and(|(_, guard)| crate::control_flow::awaiting::awaits(guard))
    }) || matches!(fall, chain::Fallthrough::CatchAll(f) if f.is_async);
    let mut keys: Vec<(String, Written)> = Vec::new();
    for arm in written_arms.iter().copied() {
        // An `|` pattern writes one body for several variants; each gets its own
        // arm with the same body, bound through its own payload.
        for case in arms::cases_of(&arm.pat) {
            let Some((variant, fields)) = payload_of(case) else {
                // A catch-all is lowered before this point, so a pattern with
                // no variant here is one the runtime's match has no key for at
                // all — a literal, a slice, a tuple against an enum — and the
                // arm would otherwise vanish without a word.
                t.report_match_gap(
                    match_expr,
                    "an arm of this match names no variant, and the runtime's match dispatches \
                     on the variant name, so the arm is not written and its body never runs",
                );
                continue;
            };
            if contested.contains(&variant) {
                let param = chain_params
                    .iter()
                    .find(|(name, _)| *name == variant)
                    .map(|(_, param)| param.clone())
                    .expect("every chained variant is given a parameter");
                let link = arms::translate_link(
                    case, &variant, &param, &fields, arm, t, match_expr, position, produces,
                    takes, scrutinee_ty.as_ref(), any_async,
                );
                match keys.iter_mut().find(|(name, _)| *name == variant) {
                    Some((_, Written::Chain { links, .. })) => links.push(link),
                    Some(_) => unreachable!("a contested variant is only ever written as a chain"),
                    None => keys.push((
                        variant.clone(),
                        Written::Chain { links: vec![link], param, has_payload: !fields.is_empty() },
                    )),
                }
                continue;
            }
            if keys.iter().any(|(name, _)| *name == variant) {
                t.report_match_gap(
                    match_expr,
                    format!(
                        "a second arm names `{}`, and the runtime's match dispatches on the \
                         variant alone, so only the first of them can run",
                        variant
                    ),
                );
                continue;
            }
            let text = arms::translate_arm(
                case, &variant, &fields, arm, t, match_expr, position, produces, takes,
                scrutinee_ty.as_ref(), any_async,
            );
            keys.push((variant, Written::Arm(text)));
        }
    }

    let mut out = format!("{}.{}{}({{\n", scrutinee, method, out_ty);
    for (variant, written) in keys {
        match written {
            Written::Arm(text) => out.push_str(&text),
            Written::Chain { links, param, has_payload } => {
                let body = chain::write(links, fall, &variant, has_payload, &param, takes, t);
                let keyword = if any_async { "async " } else { "" };
                let head = if has_payload {
                    format!("  {}: {}({}) => ", variant, keyword, param)
                } else {
                    format!("  {}: {}() => ", variant, keyword)
                };
                out.push_str(&format!("{}{{\n{}  }},\n", head, indent(&indent(&body))));
            }
        }
    }
    out.push_str("})");
    if any_async { format!("await ({})", out) } else { out }
}

/// One key of the runtime's match, before the keys are written out in order.
enum Written {
    /// The one arm that names this variant, already rendered.
    Arm(String),
    /// The arms that name it, to be written as the chain Rust tries.
    Chain { links: Vec<chain::Link>, param: String, has_payload: bool },
}

#[cfg(test)]
mod exit_tests {
    use crate::testing::Fixture;

    /// A `?` or a `return` inside a match that stands as a STATEMENT within a
    /// lifted arm has nowhere to travel: the arm hands its own value back
    /// through the sentinel, and a statement's value is discarded on the spot.
    /// So the test is written after the nested match too — and it hands the
    /// sentinel ON rather than unwrapping it, because a `return` there leaves
    /// the arm and not the function. Without it ankql's `generate_expr_sql`
    /// dropped every `Err` its inner match produced, and the `Result.Err` and
    /// its payload leaked.
    #[test]
    fn a_nested_statement_match_hands_its_exit_on() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct Token { pub n: usize }\n\
             pub enum Inner { Good, Bad }\n\
             pub enum Outer { One(Token), Two }\n\
             pub fn run(o: Outer, inner: &Inner, out: &mut String) -> Result<(), String> {\n\
               match o {\n\
                 Outer::One(t) => {\n\
                   match inner {\n\
                     Inner::Good => { out.push('g'); }\n\
                     Inner::Bad => { return Err(\"bad\".to_string()); }\n\
                   }\n\
                   out.push('1');\n\
                   drop(t);\n\
                 }\n\
                 Outer::Two => { out.push('2'); }\n\
               }\n\
               Ok(())\n\
             }",
        )]);
        let ts = f.translated_method("lib.rs", "run");
        // The inner match's sentinel is tested, and passed on whole.
        assert!(
            ts.contains("?.$jump === 'return') return _m1;")
                || ts.contains("?.$jump === 'return') return _m0;"),
            "the nested match's exit is handed on:\n{}",
            ts
        );
        // The outer one unwraps it, because there the `return` is the
        // function's.
        assert!(ts.contains("$jump === 'return') return (_m2 as any).$value;"), "{}", ts);
    }
}
