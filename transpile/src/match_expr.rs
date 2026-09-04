//! Match expression translation — Rust match → TS patterns
//!
//! Handles Option match (Some/None → null checks), Result match (Ok/Err → try/catch),
//! and enum match (variants → .match({}) pattern).

use crate::name_map;
use crate::body::{translate_pat, indent, BodyTranslator};

/// Translate a match expression in return position (adds return to each arm)
pub fn translate_match_returning(match_expr: &syn::ExprMatch, t: &BodyTranslator) -> String {
    let scrutinee = scrutinee_of(match_expr, t);

    if let Some(written) = guarded(&scrutinee, match_expr, t, Position::Returning) {
        return written;
    }
    if is_option_match(&match_expr.arms) {
        return translate_option_match_returning(&scrutinee, match_expr, t);
    }
    if is_result_match(&match_expr.arms) {
        return translate_result_match(&scrutinee, match_expr, t, Position::Returning);
    }
    if looks_like_enum_match(&match_expr.arms) {
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

/// One arm's body, written for the position the match stands in.
fn arm_body(body: &syn::Expr, t: &BodyTranslator, position: Position) -> String {
    match position {
        Position::Statement => t.expr(body),
        Position::Returning => crate::control_flow::translate_expr_in_return_position(body, t),
    }
}

/// Option match with return in each branch
fn translate_option_match_returning(scrutinee: &str, match_expr: &syn::ExprMatch, t: &BodyTranslator) -> String {
    let arms = &match_expr.arms;
    let scrutinee_ty = t.scrutinee_type(&match_expr.expr);
    let mut some_arm = None;
    let mut none_arm = None;

    for arm in arms {
        let arm_type = match &arm.pat {
            syn::Pat::TupleStruct(ts) => ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default(),
            syn::Pat::Ident(ident) => ident.ident.to_string(),
            syn::Pat::Path(path) => path.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default(),
            syn::Pat::Wild(_) => "_".to_string(),
            _ => String::new(),
        };

        match arm_type.as_str() {
            "Some" => {
                let var_name = if let syn::Pat::TupleStruct(ts) = &arm.pat {
                    ts.elems.first().map(translate_pat).unwrap_or_else(|| "v".to_string())
                } else { "v".to_string() };
                let _arm = t.enter_pattern(&arm.pat, scrutinee_ty.as_ref());
                some_arm = Some((var_name, t.expr(&arm.body)));
            }
            "None" | "_" => { none_arm = Some(t.expr(&arm.body)); }
            _ => {}
        }
    }

    match (some_arm, none_arm) {
        (Some((var, some_body)), Some(none_body)) => {
            format!("if ({} != null) {{\n  const {} = {};\n  return {};\n}} else {{\n  return {};\n}}",
                scrutinee, var, scrutinee, some_body, none_body)
        }
        (Some((var, some_body)), None) => {
            format!("if ({} != null) {{\n  const {} = {};\n  return {};\n}}",
                scrutinee, var, scrutinee, some_body)
        }
        (None, Some(none_body)) => {
            format!("if ({} == null) {{\n  return {};\n}}", scrutinee, none_body)
        }
        _ => format!("return /* match {} */;", scrutinee),
    }
}

/// Translate a match expression
pub fn translate_match(match_expr: &syn::ExprMatch, t: &BodyTranslator) -> String {
    let scrutinee = scrutinee_of(match_expr, t);

    if let Some(written) = guarded(&scrutinee, match_expr, t, Position::Statement) {
        return written;
    }
    if is_option_match(&match_expr.arms) {
        return translate_option_match(&scrutinee, match_expr, t);
    }
    if is_result_match(&match_expr.arms) {
        return translate_result_match(&scrutinee, match_expr, t, Position::Statement);
    }
    if looks_like_enum_match(&match_expr.arms) {
        return translate_enum_match(&scrutinee, match_expr, t, Position::Statement);
    }

    translate_value_match(&scrutinee, match_expr, t, Position::Statement)
}

/// A match with a guard, written the one way a guard can be written.
///
/// The runtime's `match` and `intoMatch` dispatch on the variant alone: an arm
/// whose guard failed has nowhere to fall through to, and two arms naming one
/// variant collide on one key, which JavaScript resolves by keeping the last.
/// So a guarded match that only *reads* its subject is written as the if-chain
/// below instead, which tries the arms in turn — the tests `pattern_test`
/// writes for a nullable, for an enum variant and for a literal are the same
/// borrowing reads either way.
///
/// Two shapes have no such form and are reported rather than guessed at. A
/// guarded `Result` arm reads its payload with `unwrap()`, which takes the
/// wrapper apart, so the arm below it would be reading a value that is already
/// gone. A guarded match that hands its payload over needs `intoMatch` to mark
/// the subject moved, and `intoMatch` runs its arm inside a function a
/// fall-through cannot leave.
fn guarded(
    scrutinee: &str,
    match_expr: &syn::ExprMatch,
    t: &BodyTranslator,
    position: Position,
) -> Option<String> {
    if !match_expr.arms.iter().any(|arm| arm.guard.is_some()) {
        return None;
    }
    if is_result_match(&match_expr.arms) {
        t.report_match_gap(
            match_expr,
            "an arm of this `Result` match has a guard, and reading the payload to test it \
             takes the wrapper apart, so the arm below it cannot be tried; the guard is \
             dropped and its arm runs unconditionally",
        );
        return None;
    }
    if t.match_takes(match_expr) == crate::ownership::scrutinee::Takes::Payload {
        t.report_match_gap(
            match_expr,
            "an arm of this `match` has a guard and the match hands its payload to the arms, \
             which needs `intoMatch` to mark the subject moved — and an arm of `intoMatch` is \
             a function a failed guard cannot fall out of; the guard is dropped",
        );
        return None;
    }
    Some(translate_value_match(scrutinee, match_expr, t, position))
}

/// A `match` on something the runtime has no `match` of its own for — a number,
/// a string, a tuple — as the if/else chain the arms describe.
///
/// Each arm's pattern becomes a test against the scrutinee plus the names it
/// binds. Writing the *pattern* where the test belongs emitted
/// `if (/* pat literal */)`, which does not parse and matched nothing.
fn translate_value_match(
    scrutinee: &str,
    match_expr: &syn::ExprMatch,
    t: &BodyTranslator,
    position: Position,
) -> String {
    let scrutinee_ty = t.scrutinee_type(&match_expr.expr);
    let (subject, declaration) = subject_of(scrutinee, t);
    let arms: Vec<Arm> = match_expr
        .arms
        .iter()
        .map(|arm| written_arm(arm, &subject, scrutinee_ty.as_ref(), t, position))
        .collect();
    // A guard reads the names its own pattern bound, and a guard that fails
    // hands the subject to the arm below it. An `else if` chain carries only
    // one of those two at a time, because an arm's bindings live inside the
    // block its test opens, so a match with a guard is written the other way.
    if arms.iter().any(|arm| arm.guard.is_some()) {
        return format!("{}{}", declaration, tested_in_turn(&arms, t));
    }
    let mut out = declaration;
    for (i, arm) in arms.iter().enumerate() {
        let block = indent(&format!("{}{}{}", arm.flags, arm.bind, arm.body));
        // Rust's match is exhaustive, so the last arm runs whenever nothing
        // above it matched and its own test is redundant. Writing it as one
        // more `else if` left the chain able to fall off its end and hand back
        // `undefined`, which the function's return type does not admit:
        // `match flag { true => .., false => .. }` needs no `_` arm in Rust and
        // had no `else` here.
        let catch_all = arm.test == "true" || i + 1 == arms.len();
        let head = match (i, catch_all) {
            // An arm that matches everything and stands first is the whole
            // match; there is nothing left to test.
            (0, true) => String::new(),
            (0, false) => format!("if ({}) ", arm.test),
            (_, true) => " else ".to_string(),
            (_, false) => format!(" else if ({}) ", arm.test),
        };
        out.push_str(&format!("{}{{\n{}}}", head, block));
    }
    out
}

/// One arm of a value match, written out in the pieces the two forms below
/// arrange differently.
struct Arm {
    /// What asks whether the subject matches this arm's pattern, and `"true"`
    /// where the pattern matches anything.
    test: String,
    /// The declarations the names this pattern binds need. A guard reads them,
    /// so they stand above it.
    bind: String,
    /// What runs before the guard is tested: the guard's own temporaries
    /// declared, and released again once the test has read them.
    before: String,
    /// The guard's value, where the arm has a guard.
    guard: Option<String>,
    /// The drop flags a local this arm hands away needs set.
    flags: String,
    /// The arm's body, with what it lifted out of itself declared inside it.
    body: String,
}

/// Write one arm out.
fn written_arm(
    arm: &syn::Arm,
    subject: &str,
    scrutinee_ty: Option<&crate::ty::Ty>,
    t: &BodyTranslator,
    position: Position,
) -> Arm {
    let _bindings = t.enter_pattern(&arm.pat, scrutinee_ty);
    let (test, bind) = t.pattern_test(subject, &arm.pat);
    // A guard is its own temporary scope: Rust releases what the guard took to
    // make its test before the arm's body runs and before the next arm is
    // tried, exactly as it does for the condition of an `if`.
    let (before, guard) = match &arm.guard {
        Some((_, guard)) => {
            let (written, lifted) = t.with_own_hoists(|| t.expr(guard));
            let (written, before) = t.settle_condition(written, &lifted);
            (before, Some(written))
        }
        None => (String::new(), None),
    };
    let (body, lifted) = t.with_own_hoists(|| arm_body(&arm.body, t, position));
    drop(_bindings);
    // A local this arm hands away sets its drop flag here — the same line the
    // enclosing block would have written had the arm been a statement of it.
    // Without it the `finally` released a value the arm had already given away.
    let flags = t.flag_sets_for(&arm.body);
    Arm {
        test,
        bind,
        before,
        guard,
        flags,
        body: crate::ownership::hoisted(&format!("{}\n", body), &lifted),
    }
}

/// The arms of a guarded match, tried in turn inside a labelled block that the
/// arm which matched leaves.
///
/// Each arm's bindings and its guard's temporaries stand inside the block its
/// own test opens, which is what lets the guard read the one and release the
/// other; leaving the block is what stops the arms below it from being tried.
fn tested_in_turn(arms: &[Arm], t: &BodyTranslator) -> String {
    let label = t.fresh_hoist("_match");
    let mut inner = String::new();
    for arm in arms {
        // Leaving the block is what stops the arms below from being tried. A
        // body that returns or throws has made that jump for itself.
        let leaving = if leaves_the_arm(&arm.body) {
            String::new()
        } else {
            format!("break {};\n", label)
        };
        let matched = match &arm.guard {
            Some(guard) => format!(
                "{}{}if ({}) {{\n{}}}\n",
                arm.bind,
                arm.before,
                guard,
                indent(&format!("{}{}{}", arm.flags, arm.body, leaving))
            ),
            None => format!("{}{}{}{}", arm.flags, arm.bind, arm.body, leaving),
        };
        // A pattern that matches anything still opens a block of its own: the
        // names it binds belong to this arm and to no arm written after it.
        if arm.test == "true" {
            inner.push_str(&format!("{{\n{}}}\n", indent(&matched)));
        } else {
            inner.push_str(&format!("if ({}) {{\n{}}}\n", arm.test, indent(&matched)));
        }
    }
    format!("{}: {{\n{}}}", label, indent(&inner))
}

/// Does this arm body leave the function by itself, so that nothing after it
/// in the arm would run?
fn leaves_the_arm(body: &str) -> bool {
    let last = body.trim_end().lines().last().unwrap_or_default().trim_start();
    last.starts_with("return ") || last.starts_with("return;") || last.starts_with("throw ")
}

/// A name the arms can test against, and the declaration that gives it one.
///
/// A scrutinee that is already a name is tested where it stands; anything else
/// is read once, because Rust evaluates it once and the arms each test it.
fn subject_of(scrutinee: &str, t: &BodyTranslator) -> (String, String) {
    let is_name = !scrutinee.is_empty()
        && scrutinee
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '$' || c == '.');
    if is_name {
        return (scrutinee.to_string(), String::new());
    }
    let subject = t.fresh_temp();
    let declaration = format!("const {} = {};\n", subject, scrutinee);
    (subject, declaration)
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

fn translate_option_match(scrutinee: &str, match_expr: &syn::ExprMatch, t: &BodyTranslator) -> String {
    let arms = &match_expr.arms;
    let scrutinee_ty = t.scrutinee_type(&match_expr.expr);
    let mut some_arm = None;
    let mut none_arm = None;

    for arm in arms {
        let arm_type = match &arm.pat {
            syn::Pat::TupleStruct(ts) => ts.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default(),
            syn::Pat::Ident(ident) => ident.ident.to_string(),
            syn::Pat::Path(path) => path.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default(),
            syn::Pat::Wild(_) => "_".to_string(),
            _ => String::new(),
        };

        match arm_type.as_str() {
            "Some" => {
                let var_name = if let syn::Pat::TupleStruct(ts) = &arm.pat {
                    ts.elems.first().map(translate_pat).unwrap_or_else(|| "v".to_string())
                } else {
                    "v".to_string()
                };
                let _arm = t.enter_pattern(&arm.pat, scrutinee_ty.as_ref());
                some_arm = Some((var_name, t.expr(&arm.body)));
            }
            "None" | "_" => {
                none_arm = Some(t.expr(&arm.body));
            }
            _ => {}
        }
    }

    match (some_arm, none_arm) {
        (Some((var, some_body)), Some(none_body)) => {
            format!("if ({} != null) {{\n  const {} = {};\n{}\n}} else {{\n{}\n}}",
                scrutinee, var, scrutinee, indent(&some_body), indent(&none_body))
        }
        (Some((var, some_body)), None) => {
            format!("if ({} != null) {{\n  const {} = {};\n{}\n}}",
                scrutinee, var, scrutinee, indent(&some_body))
        }
        (None, Some(none_body)) => {
            format!("if ({} == null) {{\n{}\n}}", scrutinee, indent(&none_body))
        }
        _ => format!("/* match {} */", scrutinee),
    }
}

fn is_result_match(arms: &[syn::Arm]) -> bool {
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
fn translate_result_match(
    scrutinee: &str,
    match_expr: &syn::ExprMatch,
    t: &BodyTranslator,
    position: Position,
) -> String {
    let scrutinee_ty = t.scrutinee_type(&match_expr.expr);
    let (subject, declaration) = subject_of(scrutinee, t);
    let mut ok = None;
    let mut err = None;
    for arm in &match_expr.arms {
        let variant = variant_named(&arm.pat);
        let reader = match variant.as_deref() {
            Some("Ok") => "unwrap",
            Some("Err") => "unwrapErr",
            // `_ => ..` stands for whichever branch has no arm of its own.
            _ if matches!(arm.pat, syn::Pat::Wild(_) | syn::Pat::Ident(_)) => {
                let branch = render_result_arm(&subject, arm, "unwrap", t, position, scrutinee_ty.as_ref());
                if ok.is_none() { ok = Some(branch.clone()); }
                if err.is_none() {
                    err = Some(render_result_arm(&subject, arm, "unwrapErr", t, position, scrutinee_ty.as_ref()));
                }
                continue;
            }
            _ => continue,
        };
        let branch = render_result_arm(&subject, arm, reader, t, position, scrutinee_ty.as_ref());
        match reader {
            "unwrap" => ok = Some(branch),
            _ => err = Some(branch),
        }
    }
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

/// One side of a `Result` match: the consuming read that takes the wrapper
/// apart, the name the arm binds it to, and what the arm then does.
fn render_result_arm(
    subject: &str,
    arm: &syn::Arm,
    reader: &str,
    t: &BodyTranslator,
    position: Position,
    scrutinee_ty: Option<&crate::ty::Ty>,
) -> String {
    let bound = payload_binding(&arm.pat);
    let _bindings = t.enter_pattern(&arm.pat, scrutinee_ty);
    let name = bound.clone().unwrap_or_else(|| t.fresh_temp());
    let owned = t.claim_bindings(
        std::slice::from_ref(&name),
        std::slice::from_ref(&syn::Stmt::Expr(arm.body.as_ref().clone(), None)),
    );
    let (body, lifted) = t.with_own_hoists(|| arm_body(&arm.body, t, position));
    drop(_bindings);
    let flags = t.flag_sets_for(&arm.body);
    let inner = crate::ownership::hoisted(&format!("{}\n", body), &lifted);
    format!(
        "const {} = {}.{}();\n{}",
        name,
        subject,
        reader,
        t.wrap_bindings(&owned, format!("{}{}", flags, inner))
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
    let scrutinee_ty = t.scrutinee_type(&match_expr.expr);
    let takes = t.match_takes(match_expr);
    let method = match takes {
        crate::ownership::scrutinee::Takes::Payload => "intoMatch",
        crate::ownership::scrutinee::Takes::Nothing => "match",
    };
    let mut out = format!("{}.{}({{\n", scrutinee, method);

    // One key per variant. Two arms naming one variant — which Rust reads in
    // order and this form cannot — used to emit the same key twice, and
    // JavaScript keeps the last of those, so the arm the source wrote first
    // never ran.
    let mut written: Vec<String> = Vec::new();
    for arm in &match_expr.arms {
        // An `|` pattern writes one body for several variants; each gets its own
        // arm with the same body, bound through its own payload.
        let cases: Vec<&syn::Pat> = match &arm.pat {
            syn::Pat::Or(or_pat) => or_pat.cases.iter().collect(),
            other => vec![other],
        };
        for case in cases {
            let Some((variant, fields)) = payload_of(case) else {
                continue;
            };
            if written.contains(&variant) {
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
            written.push(variant.clone());
            let _bindings = t.enter_pattern(case, scrutinee_ty.as_ref());
            // Where the enum handed its payload over, the arm owns what the
            // pattern named and releases it however the arm is left.
            let names: Vec<String> = match takes {
                crate::ownership::scrutinee::Takes::Payload => fields
                    .iter()
                    .map(|(local, _)| local.clone())
                    .filter(|local| local != "_")
                    .collect(),
                crate::ownership::scrutinee::Takes::Nothing => Vec::new(),
            };
            let owned = t.claim_bindings(
                &names,
                std::slice::from_ref(&syn::Stmt::Expr(arm.body.as_ref().clone(), None)),
            );
            // An arm is an arrow function, so what the arm's own expression
            // lifted out of itself stays inside it: the declaration names
            // values the arm's payload produced, which do not exist outside.
            // A block body is written as the arrow's own statements, with the
            // `return` on its tail; as an immediately-called function it
            // computed the arm's value and threw it away, and a `return`
            // written inside it left the inner function rather than the
            // enclosing one.
            let (body, lifted) = t.with_own_hoists(|| t.statements(&arm.body));
            let body = body.trim_end().to_string();
            drop(_bindings);
            // An arm is an arrow function, so a `?` inside one returns from the
            // arm. Where the match is the enclosing function's value that is
            // exactly right — the arm's `Result` is what the function returns —
            // and where it is a statement it is not, and nobody sees the error.
            if position == Position::Statement
                && lifted.iter().any(|h| h.declaration.contains("return "))
            {
                t.report_match_gap(
                    match_expr,
                    "an arm leaves early, and the arm is an arrow function whose `return` \
                     leaves the arm rather than the function, so nobody sees the error it \
                     left with",
                );
            }
            // An arm is an arrow function, so a local this arm hands away sets
            // its drop flag here — the same line the enclosing block would have
            // written had the arm been a statement of it.
            let flags = t.flag_sets_for(&arm.body);
            out.push_str(&render_arm(
                &variant, &fields, &body, &flags, &owned, &lifted, t,
            ));
        }
    }

    out.push_str("})");
    out
}

/// The variant a pattern names and the payload slots it takes out of it, as
/// (local name, accessor within the arm's payload).
fn payload_of(pat: &syn::Pat) -> Option<(String, Vec<(String, String)>)> {
    match pat {
        syn::Pat::TupleStruct(ts) => {
            let variant = ts.path.segments.last()?.ident.to_string();
            let fields = ts
                .elems
                .iter()
                .enumerate()
                .map(|(i, p)| (translate_pat(p), format!("_{}", i)))
                .collect();
            Some((variant, fields))
        }
        syn::Pat::Struct(s) => {
            let variant = s.path.segments.last()?.ident.to_string();
            let fields = s
                .fields
                .iter()
                .map(|f| {
                    let member = match &f.member {
                        syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                        syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                    };
                    (translate_pat(&f.pat), member)
                })
                .collect();
            Some((variant, fields))
        }
        syn::Pat::Path(p) => Some((p.path.segments.last()?.ident.to_string(), Vec::new())),
        _ => None,
    }
}

/// One arm of a `.match({..})`.
///
/// The payload's names are declared inside the arm, from the value the arm is
/// handed. They used to be substituted into the rendered TypeScript by walking
/// its characters, which could not tell a binding from the same word inside a
/// string literal or a comment, and knew nothing of a name shadowed further in.
fn render_arm(
    variant: &str,
    fields: &[(String, String)],
    body: &str,
    flags: &str,
    owned: &[crate::ownership::Owned],
    lifted: &[crate::ownership::Hoist],
    t: &BodyTranslator,
) -> String {
    // The arm's parameter must not collide with a name the pattern binds.
    let param = if fields.iter().any(|(local, _)| local == "v") { "_v" } else { "v" };
    let head = if fields.is_empty() {
        format!("  {}: () => ", variant)
    } else {
        format!("  {}: ({}) => ", variant, param)
    };
    let mut bindings = String::from(flags);
    for (local, accessor) in fields {
        if local == "_" {
            continue;
        }
        bindings.push_str(&format!("const {} = {}.{};\n", local, param, accessor));
    }
    if owned.is_empty() && lifted.is_empty() {
        return format!("{}{},\n", head, as_arm_value(body, &bindings));
    }
    // An arm that owns what it was handed, or that lifted a declaration out of
    // its own body, is always a block: the release goes in a `finally`, so the
    // arm cannot be the bare expression form.
    let inner = t.wrap_bindings(
        owned,
        crate::ownership::hoisted(&arm_statements(body), lifted),
    );
    format!(
        "{}{{\n{}  }},\n",
        head,
        indent(&indent(&format!("{}{}", bindings, inner)))
    )
}

/// An arm body as statements: its own control flow where it has some, and
/// otherwise the `return` that makes its value the arm's.
fn arm_statements(body: &str) -> String {
    if is_statements(body) {
        return format!("{}\n", body);
    }
    format!("return {};\n", body)
}

/// Does this body already read as a run of statements?
fn is_statements(body: &str) -> bool {
    body.starts_with("if ")
        || body.starts_with("for ")
        || body.starts_with("while ")
        || body.starts_with("return ")
        || body.starts_with("throw ")
        || body.starts_with('{')
        || body.contains(";\n")
}

/// An arm's value: its declarations, then the body. A body that is already a
/// sequence of statements keeps its own control flow; anything else is the
/// value the arm produces.
fn as_arm_value(body: &str, bindings: &str) -> String {
    let statements = is_statements(body);
    // A tuple literal confuses TypeScript's inference across arms: `match`
    // takes its result type from the first arm it reads, and `[null, events]`
    // makes every later arm an error against it.
    let value = if body.starts_with('[') {
        format!("{} as any", body)
    } else {
        body.to_string()
    };
    if bindings.is_empty() && !statements {
        return value;
    }
    let tail = if statements {
        body.to_string()
    } else {
        format!("return {};", value)
    };
    // The arm sits two spaces in, so its block's contents sit at four and the
    // closing brace lines up with the arm.
    format!(
        "{{\n{}  }}",
        indent(&indent(&format!("{}{}\n", bindings, tail)))
    )
}
