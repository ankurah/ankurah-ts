//! A `match` the runtime's own `match` cannot dispatch: a number, a string, a
//! tuple, an `Ordering`, a borrowed enum with a guard.
//!
//! For: `.match({..})` has one key per variant and dispatches on the variant
//! name, so a subject with no variants has no key at all, and a guard cannot be
//! part of a key. What is written instead is the if-chain the arms describe:
//! each arm's pattern becomes a test against the subject, the names it binds
//! stand in the branch that test opens, and its guard is a second test made
//! there. The chain reads the subject and never marks it moved, which is what
//! a borrow needs and what a consuming match cannot have — `guarded` sends
//! those elsewhere.

use super::arms::arm_body;
use super::{indent, Position};
use crate::body::BodyTranslator;

/// A `match` on something the runtime has no `match` of its own for — a number,
/// a string, a tuple — as the if/else chain the arms describe.
///
/// Each arm's pattern becomes a test against the scrutinee plus the names it
/// binds. Writing the *pattern* where the test belongs emitted
/// `if (/* pat literal */)`, which does not parse and matched nothing.
pub(super) fn translate_value_match(
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
    // What the subject's OWN release no longer covers. A consuming match marks
    // the subject moved and writes no release for it, so a tuple pattern that
    // names only some of its elements left the rest with no owner at all
    // (H2/I1): `match pair { (a, _) => .. }` released `a` and let `pair[1]`
    // reach the collector. The port writes a tuple as an array and knows every
    // position of it, so the arm releases the positions its pattern did not
    // name.
    let consumes = t.match_takes(match_expr) == crate::ownership::scrutinee::Takes::Payload;
    let arms: Vec<Arm> = match_expr
        .arms
        .iter()
        .map(|arm| {
            written_arm(
                arm,
                &subject,
                &match_expr.expr,
                scrutinee_ty.as_ref(),
                t,
                position,
                consumes.then(|| unowned_elements(&arm.pat, scrutinee_ty.as_ref(), t)).unwrap_or_default(),
            )
        })
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
    /// Does every path out of the body leave the block the arms are tried in,
    /// so that nothing needs to jump over the arms below it? Carried from the
    /// lowering (K2) rather than read back out of the text.
    leaves: bool,
}

/// Write one arm out.
#[allow(clippy::too_many_arguments)]
fn written_arm(
    arm: &syn::Arm,
    subject: &str,
    subject_expr: &syn::Expr,
    scrutinee_ty: Option<&crate::ty::Ty>,
    t: &BodyTranslator,
    position: Position,
    unowned: Vec<usize>,
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
    let ((body, leaves), lifted) = t.with_own_hoists(|| arm_body(&arm.body, t, position));
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
        body: releasing_unowned_elements(
            t.wrap_bindings(&owned, crate::ownership::hoisted(&format!("{}\n", body), &lifted)),
            subject,
            &unowned,
        ),
        leaves,
    }
}

/// Which positions of a tuple subject this arm's pattern left with no owner.
///
/// Answered from the SUBJECT's type, which is where the port knows what each
/// position holds. A subject it could not read as a tuple, and a pattern with a
/// `..` in it, answer nothing — `taking::taken` refuses the same shapes where
/// the type is not in hand.
fn unowned_elements(
    pat: &syn::Pat,
    scrutinee_ty: Option<&crate::ty::Ty>,
    t: &BodyTranslator,
) -> Vec<usize> {
    let Some(crate::ty::Ty::Tuple(elements)) = scrutinee_ty.map(|ty| ty.peel_refs()) else {
        return Vec::new();
    };
    let Some(tc) = &t.types else { return Vec::new() };
    let tc = tc.borrow();
    crate::ownership::arm_takes::unowned_droppable_positions(pat, elements, &tc.probe())
        .unwrap_or_default()
}

/// The arm's body with the positions nothing named released on every path out
/// of it, which is where Rust drops them: at the end of the match, however the
/// arm leaves.
fn releasing_unowned_elements(body: String, subject: &str, unowned: &[usize]) -> String {
    if unowned.is_empty() {
        return body;
    }
    let releases: String = unowned
        .iter()
        .map(|at| format!("  dropOwned({}[{}]);\n", subject, at))
        .collect();
    format!("try {{\n{}}} finally {{\n{}}}\n", super::indent(&body), releases)
}

/// The arms of a guarded match, tried in turn inside a labelled block that the
/// arm which matched leaves.
///
/// Each arm's bindings and its guard's temporaries stand inside the block its
/// own test opens, which is what lets the guard read the one and release the
/// other; leaving the block is what stops the arms below it from being tried.
fn tested_in_turn(arms: &[Arm], t: &BodyTranslator) -> String {
    let last = arms.len().saturating_sub(1);
    let branches: Vec<super::chain::tried::Branch> = arms
        .iter()
        .enumerate()
        .map(|(at, arm)| {
            // Rust's match is exhaustive, so the LAST arm runs whenever nothing
            // above it matched and its own test is redundant. Written as one
            // more `if`, the block could fall off its end and the enclosing
            // function hand back `undefined`, which its return type does not
            // admit; and nothing stands after it to jump over.
            let exhausted = at == last && arm.guard.is_none();
            let test = (arm.test != "true" && !exhausted).then(|| arm.test.clone());
            super::chain::tried::Branch {
                test,
                // The names the pattern bound and whatever the guard lifted,
                // which stand before the guard because it reads them.
                bindings: format!("{}{}", arm.bind, arm.before),
                guard: arm.guard.as_ref().map(|guard| super::chain::tried::Guard {
                    test: guard.clone(),
                    lifted: Vec::new(),
                    release: String::new(),
                }),
                block: format!("{}{}", arm.flags, arm.body),
                leaves: arm.leaves,
            }
        })
        .collect();
    // Trimmed: this stands as one statement of the enclosing block, which adds
    // its own line ending, and the labelled form it replaced carried none.
    super::chain::tried::tried_in_turn(&branches, "", "_match", t).trim_end().to_string()
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
pub(super) fn subject_of_bound(scrutinee: &str, binds: &[String], t: &BodyTranslator) -> (String, String) {
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

