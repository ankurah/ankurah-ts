//! Match expression translation — Rust match → TS patterns
//!
//! Handles Option match (Some/None → null checks), Result match (Ok/Err → try/catch),
//! and enum match (variants → .match({}) pattern).

mod arms;
mod catch_all;
mod chain;
mod option_chain;

use crate::body::{translate_pat, indent, BodyTranslator};
use crate::control_flow::sentinel::{jumps_in, jumps_out, leaves_the_function};
use arms::{arm_statements, payload_of, render_arm, ArmParts};

/// Translate a match expression in return position (adds return to each arm)
pub fn translate_match_returning(match_expr: &syn::ExprMatch, t: &BodyTranslator) -> String {
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

/// One arm's body, written for the position the match stands in.
///
/// F3: whether a value is wanted comes from the POSITION the lowering chose,
/// never from anything read off the generated text or off an expectation left
/// standing. An arm of a statement match produces nothing, so its block is a
/// run of statements — asked as an expression, `{ if n == 0 { return .. } .. }`
/// came back an arrow function whose value was then written as a statement of
/// its own.
fn arm_body(body: &syn::Expr, t: &BodyTranslator, position: Position) -> String {
    match position {
        Position::Statement => match body {
            syn::Expr::Block(block) if block.label.is_none() => {
                t.translate_block(&block.block).trim_end().to_string()
            }
            other => t.expr(other),
        },
        Position::Returning => crate::control_flow::translate_expr_in_return_position(body, t),
    }
}

/// Translate a match expression
pub fn translate_match(match_expr: &syn::ExprMatch, t: &BodyTranslator) -> String {
    let scrutinee = scrutinee_of(match_expr, t);

    if let Some(written) = guarded(&scrutinee, match_expr, t, Position::Statement) {
        return written;
    }
    if is_option_match_typed(match_expr, t) {
        return option_chain::translate(&scrutinee, match_expr, t, Position::Statement);
    }
    if is_result_match(&match_expr.arms) {
        return translate_result_match(&scrutinee, match_expr, t, Position::Statement);
    }
    // An ordering is a number, so a `match` on one is a chain of comparisons.
    // The runtime's `.match({..})` dispatches on a variant name, and a number
    // has none.
    if t.is_ordering_value(&match_expr.expr) {
        return translate_value_match(&scrutinee, match_expr, t, Position::Statement);
    }
    if looks_like_enum_match(&match_expr.arms) {
        if let Some(written) = leaves_the_loop(&scrutinee, match_expr, t, Position::Statement) {
            return written;
        }
        if let Some(written) = tests_inside_a_variant(&scrutinee, match_expr, t, Position::Statement) {
            return written;
        }
        return translate_enum_match(&scrutinee, match_expr, t, Position::Statement);
    }

    translate_value_match(&scrutinee, match_expr, t, Position::Statement)
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
    let scrutinee_ty = t.borrowed_scrutinee_type(&match_expr.expr);
    let binds: Vec<String> = match_expr
        .arms
        .iter()
        .flat_map(|arm| crate::body::pattern_names(&arm.pat))
        .collect();
    let (subject, declaration) = subject_of_bound(scrutinee, &binds, t);
    let arms: Vec<Arm> = match_expr
        .arms
        .iter()
        .map(|arm| written_arm(arm, &subject, &match_expr.expr, scrutinee_ty.as_ref(), t, position))
        .collect();
    // A guard reads the names its own pattern bound, and a guard that fails
    // hands the subject to the arm below it. An `else if` chain carries only
    // one of those two at a time, because an arm's bindings live inside the
    // block its test opens, so a match with a guard is written the other way.
    if arms.iter().any(|arm| arm.guard.is_some()) {
        return format!("{}{}", declaration, tested_in_turn(&arms, t));
    }
    let mut out = declaration;
    let mut everything_matched = false;
    for (i, arm) in arms.iter().enumerate() {
        // An arm that matches everything is the end of the chain: Rust never
        // reaches what is written after it, and an `else` written after a block
        // that is not an `if` does not parse at all.
        if everything_matched {
            t.report_match_gap(
                match_expr,
                "an arm above this one matches everything, and Rust tries arms in order, so                  this one never runs and is not written",
            );
            break;
        }
        let block = indent(&format!("{}{}{}", arm.flags, arm.bind, arm.body));
        // Rust's match is exhaustive, so the last arm runs whenever nothing
        // above it matched and its own test is redundant. Writing it as one
        // more `else if` left the chain able to fall off its end and hand back
        // `undefined`, which the function's return type does not admit:
        // `match flag { true => .., false => .. }` needs no `_` arm in Rust and
        // had no `else` here.
        // ...but an arm whose test the translator could not write is not one
        // that matches anything: `false` is what a refused pattern answers, and
        // reading the LAST arm as a catch-all whatever its test said turned
        // "written as one that never matches" into the arm that always runs.
        let catch_all =
            arm.test == "true" || (i + 1 == arms.len() && arm.test != "false");
        let head = match (i, catch_all) {
            // An arm that matches everything and stands first is the whole
            // match; there is nothing left to test.
            (0, true) => String::new(),
            (0, false) => format!("if ({}) ", arm.test),
            (_, true) => " else ".to_string(),
            (_, false) => format!(" else if ({}) ", arm.test),
        };
        out.push_str(&format!("{}{{\n{}}}", head, block));
        everything_matched = arm.test == "true";
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
    subject_expr: &syn::Expr,
    scrutinee_ty: Option<&crate::ty::Ty>,
    t: &BodyTranslator,
    position: Position,
) -> Arm {
    let _bindings = t.enter_pattern(&arm.pat, scrutinee_ty);
    let (test, bind) = t.pattern_test(subject, &arm.pat);
    // `other => ..` moves the subject into `other` on the path this arm runs,
    // and on no other path — which is what a drop flag is for. Without the
    // flag the binding and the block both released the same value, and with a
    // guard in front of it the flag has to be set INSIDE the guard, because a
    // guard that fails hands the subject to the arm below.
    let takes_subject = crate::ownership::scrutinee::binds_whole_subject(&arm.pat);
    let subject_flag = if takes_subject { t.flag_set_for_subject(subject_expr) } else { String::new() };
    let owned = if subject_flag.is_empty() {
        Vec::new()
    } else {
        t.claim_bindings(
            &crate::body::pattern_names(&arm.pat),
            std::slice::from_ref(&syn::Stmt::Expr(arm.body.as_ref().clone(), None)),
        )
    };
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
    let flags = format!("{}{}", subject_flag, t.flag_sets_for(&arm.body));
    Arm {
        test,
        bind,
        before,
        guard,
        flags,
        body: t.wrap_bindings(&owned, crate::ownership::hoisted(&format!("{}\n", body), &lifted)),
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
/// The same, told which names the arms are about to bind.
///
/// A pattern may bind the subject's OWN name — `match b { Some(b) => b + 1 }`
/// is ordinary Rust, and a shadow there is what the source meant. `const b = b`
/// is a `ReferenceError: Cannot access 'b' before initialization`, so where an
/// arm binds the name the subject is written as, the subject is read into a
/// temporary first and the shadow declares against that.
fn subject_of_bound(scrutinee: &str, binds: &[String], t: &BodyTranslator) -> (String, String) {
    let is_name = !scrutinee.is_empty()
        && scrutinee
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '$' || c == '.');
    if is_name && !binds.iter().any(|name| name == scrutinee) {
        return (scrutinee.to_string(), String::new());
    }
    let subject = t.fresh_temp();
    let declaration = format!("const {} = {};\n", subject, scrutinee);
    (subject, declaration)
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
    let scrutinee_ty = t.borrowed_scrutinee_type(&match_expr.expr);
    let binds: Vec<String> = match_expr
        .arms
        .iter()
        .flat_map(|arm| crate::body::pattern_names(&arm.pat))
        .collect();
    let (subject, declaration) = subject_of_bound(scrutinee, &binds, t);
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
    // A borrowed `Result` is still its owner's: the payload is READ, not taken.
    let reader = match (reader, matches!(scrutinee_ty, Some(crate::ty::Ty::Ref { .. }))) {
        ("unwrap", true) => "okRef",
        ("unwrapErr", true) => "errRef",
        (owned, _) => owned,
    };
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
    let mut keys: Vec<(String, Written)> = Vec::new();
    let mut any_async = false;
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
                    takes, scrutinee_ty.as_ref(), &mut any_async,
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
                scrutinee_ty.as_ref(), &mut any_async,
            );
            keys.push((variant, Written::Arm(text)));
        }
    }

    let mut out = format!("{}.{}{}({{\n", scrutinee, method, out_ty);
    for (variant, written) in keys {
        match written {
            Written::Arm(text) => out.push_str(&text),
            Written::Chain { links, param, has_payload } => {
                // A chain awaits where any of its branches does, and the
                // catch-all's body is one of them.
                let awaits = links.iter().any(|link| link.is_async)
                    || matches!(fall, chain::Fallthrough::CatchAll(f) if f.is_async);
                let body = chain::write(links, fall, &variant, has_payload, &param, t);
                let keyword = if awaits { "async " } else { "" };
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


/// Does this body already read as a run of statements?
pub(crate) fn is_statements(body: &str) -> bool {
    body.starts_with("if ")
        || body.starts_with("for ")
        || body.starts_with("while ")
        || body.starts_with("return ")
        || body.starts_with("throw ")
        || body.starts_with('{')
        || body.contains(";\n")
        // A tail whose Rust value is `()` is written as the statement it is,
        // and a value expression never ends in a semicolon, so the semicolon
        // is what tells them apart: without this the arm put a `return` back
        // in front of it and handed back what the port's own spelling produced.
        || body.trim_end().ends_with(';')
}

/// Does this text *open* with a statement, rather than merely contain one?
///
/// `is_statements` answers the looser question — an expression written over
/// several lines contains a `;` too — and the difference matters where the
/// caller is deciding whether to write `return` in front of the text: `return
/// const _v = [` does not parse, and `return await (async () => { … })()` is
/// exactly what a value-producing macro wants.
pub(crate) fn begins_a_statement(body: &str) -> bool {
    let body = body.trim_start();
    ["const ", "let ", "var ", "if ", "for ", "while ", "do ", "return ", "throw ", "{"]
        .iter()
        .any(|opener| body.starts_with(opener))
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
