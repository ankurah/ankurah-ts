//! `match r { Ok(..) => .., Err(..) => .. }`, with more than one arm a side.
//!
//! For: a `Result` is a value here, not a thrown error, so the match is the
//! test `isOk()` and ONE consuming read on the branch that was taken —
//! `unwrap()` or `unwrapErr()`, each of which takes the wrapper apart and hands
//! back what it held. Reading the payload twice is not available, so a side
//! with several arms cannot be an `else if` chain over the `Result`.
//!
//! It can be a chain over the PAYLOAD, which is what this writes: the side
//! reads the payload once into a name, and the arms that name that variant are
//! tried against it in Rust order, each with its own inner pattern test, its
//! own guard and its own bindings. Before this, the side kept whichever arm the
//! source wrote LAST and said nothing about the rest —
//! `core/src/context.rs:187` lost `Err(RetrievalError::NoDurablePeers) if
//! cached => ()` entirely, so a cached entity with no durable peers answered an
//! error where Rust answers `Ok`.
//!
//! A side with exactly one arm and no guard is written as it always was: one
//! read, bound to the arm's own name.

use crate::body::BodyTranslator;

/// Which half of the `Result` a side is, spelled as the two reads it makes.
pub(super) struct Reader {
    /// `self` in Rust: takes the wrapper apart.
    pub owned: &'static str,
    /// `&self` in Rust: reads the payload and leaves the wrapper whole.
    pub borrowed: &'static str,
    /// Which of `Result<T, E>`'s two type arguments this side holds.
    pub argument: usize,
}

pub(super) const OK: Reader = Reader { owned: "unwrap", borrowed: "okRef", argument: 0 };
pub(super) const ERR: Reader = Reader { owned: "unwrapErr", borrowed: "errRef", argument: 1 };

/// What this side of a `Result<T, E>` holds, where the engine can name it.
///
/// The arm's name is bound to the payload the side read, and a pattern that
/// takes no name of its own is bound to a TEMPORARY — which the type context
/// has never heard of, so the disposition analysis found no type for it and
/// gave it no release. `Err(RetrievalError::NoDurablePeers) if cached => ()`
/// leaked its error every time the guard held. The side knows what it read, so
/// it says.
fn payload_type(scrutinee_ty: Option<&crate::ty::Ty>, reader: &Reader, t: &BodyTranslator) -> Option<crate::ty::Ty> {
    let mut ty = scrutinee_ty?;
    while let crate::ty::Ty::Ref { inner, .. } = ty {
        ty = inner;
    }
    let crate::ty::Ty::Named { id, args } = ty else { return None };
    let types = t.types.as_ref()?;
    let borrowed = types.borrow();
    if borrowed.probe().reg.system_type("std::result::Result") != Some(*id) {
        return None;
    }
    drop(borrowed);
    args.get(reader.argument).cloned()
}

/// Both sides of the `if (r.isOk())`, in the order the source names them.
///
/// An arm belongs to the side its variant names; a catch-all belongs to BOTH,
/// standing for whatever the arms above it left on either. The sides are
/// written in source order so that the temporaries each takes are numbered
/// where Rust evaluates them.
pub(super) fn both_sides(
    match_expr: &syn::ExprMatch,
    subject: &str,
    t: &BodyTranslator,
    position: super::Position,
    scrutinee_ty: Option<&crate::ty::Ty>,
) -> (Option<String>, Option<String>) {
    let mut ok_arms: Vec<&syn::Arm> = Vec::new();
    let mut err_arms: Vec<&syn::Arm> = Vec::new();
    for arm in &match_expr.arms {
        match super::variant_named(&arm.pat).as_deref() {
            Some("Ok") => ok_arms.push(arm),
            Some("Err") => err_arms.push(arm),
            _ if matches!(arm.pat, syn::Pat::Wild(_) | syn::Pat::Ident(_)) => {
                ok_arms.push(arm);
                err_arms.push(arm);
            }
            _ => {}
        }
    }
    let first_of = |arms: &[&syn::Arm]| {
        arms.first().map(|arm| syn::spanned::Spanned::span(&arm.pat).start())
    };
    let ok_leads = match (first_of(&ok_arms), first_of(&err_arms)) {
        (Some(ok), Some(err)) => ok <= err,
        _ => true,
    };
    if ok_leads {
        let ok = side(subject, &OK, &ok_arms, t, position, scrutinee_ty);
        let err = side(subject, &ERR, &err_arms, t, position, scrutinee_ty);
        return (ok, err);
    }
    let err = side(subject, &ERR, &err_arms, t, position, scrutinee_ty);
    let ok = side(subject, &OK, &ok_arms, t, position, scrutinee_ty);
    (ok, err)
}

/// One side of the `if (r.isOk())`, as the statements that run there.
///
/// `arms` are every arm that side can reach, in source order: the ones naming
/// its variant, and the catch-all where the source wrote one, which stands for
/// whichever values the arms above it left.
fn side(
    subject: &str,
    reader: &Reader,
    arms: &[&syn::Arm],
    t: &BodyTranslator,
    position: super::Position,
    scrutinee_ty: Option<&crate::ty::Ty>,
) -> Option<String> {
    let read = match scrutinee_ty {
        // A borrowed `Result` is still its owner's: the payload is READ, not
        // taken.
        Some(crate::ty::Ty::Ref { .. }) => reader.borrowed,
        _ => reader.owned,
    };
    let holds = payload_type(scrutinee_ty, reader, t);
    let single = arms.len() == 1 && arms[0].guard.is_none();
    let first = arms.first().copied()?;
    if single {
        let read = Read { expr: format!("{}.{}()", subject, read), ty: holds };
        let bound = super::payload_binding(&first.pat);
        let written = one_arm(&read, first, bound, "", t, position, scrutinee_ty);
        return Some(format!("{}{}", written.bindings, written.block));
    }

    // Several arms, or one with a guard: the payload is read ONCE and the arms
    // are tried against the name it was read into.
    let payload = Read { expr: t.fresh_temp(), ty: holds };
    let mut written: Vec<(Option<String>, Arm)> = Vec::new();
    let mut open = true;
    for arm in arms.iter().copied() {
        let link = link(&payload, arm, t, position, scrutinee_ty);
        let unconditional = link.0.is_none() && link.1.guard.is_none();
        written.push(link);
        if unconditional {
            open = false;
            break;
        }
    }
    // rustc proved the arms of a side exhaustive between them, so a value that
    // failed every test above matches the LAST of them — `Ok(true)` beside
    // `Ok(false)` covers a `bool`, and neither covers it alone. The port cannot
    // see that proof, so it takes the last arm's test off rather than writing a
    // refusal for a value that cannot arrive. A last arm with a GUARD is the one
    // case this cannot be said of, and that one keeps its refusal.
    if open {
        if let Some((test, arm)) = written.last_mut() {
            if arm.guard.is_none() {
                *test = None;
                open = false;
            }
        }
    }

    let branches: Vec<super::chain::tried::Branch> = written
        .into_iter()
        .map(|(test, arm)| super::chain::tried::Branch {
            test,
            bindings: arm.bindings,
            guard: arm.guard,
            block: arm.block,
            leaves: arm.leaves,
        })
        .collect();
    // Rust proves the arms of a side exhaustive between them; the port cannot
    // see that proof, and a chain that fell off its end would leave the payload
    // to nobody and hand back `undefined`. R12.
    let tail = match open {
        true => hole_in_a_side(
            "every arm of this side of the `Result` match has a test or a guard, and rustc \
             proved between them that one always holds; the port cannot see that proof, so a \
             value that fails all of them arrives here",
            &payload,
            t,
        ),
        false => String::new(),
    };
    let inner = super::chain::tried::tried_in_turn(&branches, &tail, "_arm", t);

    Some(format!("const {} = {}.{}();\n{}", payload.expr, subject, read, inner))
}

/// One arm of a side: what it declares, and what it then does.
///
/// `payload` is the expression the arm's name is bound to — the read itself
/// where the side has one arm, and the name the side read into where it has
/// several.
fn one_arm(
    payload: &Read,
    arm: &syn::Arm,
    bound: Option<String>,
    reads: &str,
    t: &BodyTranslator,
    position: super::Position,
    scrutinee_ty: Option<&crate::ty::Ty>,
) -> Arm {
    let _bindings = t.enter_pattern(&arm.pat, scrutinee_ty);
    // A pattern that takes no name still owns the payload on this branch, so it
    // is bound to a temporary and the disposition analysis gives that temporary
    // the drop the source's `_` implies — under the type the SIDE knows it read,
    // because a temporary has no declaration to look one up from.
    let name = bound.unwrap_or_else(|| t.fresh_temp());
    let body_stmt = [syn::Stmt::Expr(arm.body.as_ref().clone(), None)];
    let holds = |looked_up: &str| {
        t.types
            .as_ref()
            .and_then(|tc| tc.borrow().lookup(looked_up))
            .or_else(|| payload.ty.clone())
    };
    // The side READ the payload out of the `Result` — `unwrap()` and
    // `unwrapErr()` both hand it over and leave nothing behind — so this arm
    // owns it whether or not the engine can name its type. Skipping the release
    // where the type did not resolve abandoned the error four corpus sites
    // take: `const _v2 = _v1; return Result.Ok(null)`.
    let owned = t.claim_bindings_as(
        std::slice::from_ref(&name),
        &holds,
        crate::ownership::Drops::Cascade,
        &body_stmt,
    );
    // What the arm owes if its GUARD throws is a different question from what
    // its BODY owes: the guard is made before a statement of the body has run,
    // so a payload the body goes on to move is still the arm's here. Asked with
    // an empty body, which is what has run at that point.
    let owed_by_the_guard =
        t.claim_bindings_as(std::slice::from_ref(&name), &holds, crate::ownership::Drops::Cascade, &[]);
    // Inside the pattern's scope, because the guard reads the names the pattern
    // bound; and before the body, because Rust tests the guard first. What the
    // test lifts out of itself stays with it, and what the arm owes if the test
    // THROWS is the payload the side already read out of the `Result`.
    let guard = arm.guard.as_ref().map(|(_, guard)| {
        let (test, lifted) = t.with_own_hoists(|| t.expr(guard));
        super::chain::tried::Guard {
            test,
            lifted,
            release: released_payload(&name, &owed_by_the_guard),
        }
    });
    let (body, lifted) = t.with_own_hoists(|| super::arms::arm_body(&arm.body, t, position));
    drop(_bindings);
    let flags = t.flag_sets_for(&arm.body);
    let inner = crate::ownership::hoisted(&format!("{}\n", body), &lifted);
    let statements = format!("{}{}", flags, inner);
    Arm {
        bindings: format!("const {} = {};\n{}", name, payload.expr, reads),
        guard,
        // Asked of the body itself rather than of what `wrap_bindings` puts
        // around it: a `finally` that releases the arm's binding still runs on
        // the way out of a `return`.
        leaves: super::leaves_the_arm(&statements),
        block: t.wrap_bindings(&owned, statements),
    }
}

/// What a side read, and what it read it INTO.
struct Read {
    /// The expression an arm's name is bound to: the read itself where the side
    /// has one arm, and the name it read into where it has several.
    expr: String,
    /// What the side holds, where the engine can name it. A pattern that takes
    /// no name is bound to a temporary the type context has never heard of, so
    /// the disposition analysis asks here instead.
    ty: Option<crate::ty::Ty>,
}

/// One arm of a side, written out.
struct Arm {
    /// The name the arm binds the payload to.
    bindings: String,
    /// The arm's guard, read against the names above.
    guard: Option<super::chain::tried::Guard>,
    /// The body and what it owes around it.
    block: String,
    /// Whether the body leaves by itself, so the chain needs no `break`.
    leaves: bool,
}

/// One link of a side's chain: the test its inner pattern makes of the payload,
/// what it declares, its body, and whether the body leaves by itself.
fn link(
    payload: &Read,
    arm: &syn::Arm,
    t: &BodyTranslator,
    position: super::Position,
    scrutinee_ty: Option<&crate::ty::Ty>,
) -> (Option<String>, Arm) {
    let written = |inner: &syn::Pat| {
        let (test, reads) = t.pattern_test(&payload.expr, inner);
        let test = (test.trim() != "true" && !test.starts_with("unsupported(")).then_some(test);
        (test, reads)
    };
    match inner_test(arm) {
        Inner::Whole => (
            None,
            one_arm(payload, arm, super::payload_binding(&arm.pat), "", t, position, scrutinee_ty),
        ),
        Inner::Tests(inner) => {
            let (test, _) = written(&inner);
            (test, one_arm(payload, arm, None, "", t, position, scrutinee_ty))
        }
        // `Ok(Some(state))`: the port writes `Option<T>` as `T | null`, so the
        // inner name IS the payload and there is no wrapper left behind. The arm
        // binds it under that name, which is what the arm's body reads.
        Inner::TestsAndTakesAll { test: inner, name } => {
            let (test, _) = written(&inner);
            (test, one_arm(payload, arm, Some(name), "", t, position, scrutinee_ty))
        }
        Inner::TestsAndBindsPart(inner) => {
            let (test, reads) = written(&inner);
            // Reading a part out of the payload is a COPY where the part is not
            // droppable — a number, a string — so the payload is still whole and
            // the arm's temporary releases all of it. Where a part IS droppable,
            // the arm has taken it and the port has no way to release an object
            // minus one field: R12, and the refusal stands in the BRANCH so a
            // value this pattern does not match still reaches the arms below.
            if takes_something_droppable(&inner, arm, t, scrutinee_ty) {
                let what = "an arm of this `Result` match tests INSIDE the payload and takes a \
                            DROPPABLE name out of it, and the port cannot both take a name out of \
                            a payload and release what is left of it here";
                t.fallback(syn::spanned::Spanned::span(&arm.pat), what);
                // The refusal stands where the ARM would have run, so it keeps
                // the arm's guard: a value whose guard fails belongs to the arm
                // below, and throwing for it refused a case the port can write.
                let guard = arm.guard.as_ref().map(|(_, guard)| {
                    let _bindings = t.enter_pattern(&arm.pat, scrutinee_ty);
                    let (test, lifted) = t.with_own_hoists(|| t.expr(guard));
                    drop(_bindings);
                    super::chain::tried::Guard { test, lifted, release: payload_release(payload, t) }
                });
                return (
                    test,
                    Arm {
                        bindings: String::new(),
                        guard,
                        block: hole_in_a_side(what, payload, t),
                        leaves: true,
                    },
                );
            }
            (test, one_arm(payload, arm, None, &reads, t, position, scrutinee_ty))
        }
    }
}

/// A HOLE written where a side holds the payload, with what it owes first.
///
/// R12 says a hole throws where the branch would have run; it does not say the
/// branch may abandon what it was handed. The side has already taken the
/// `Result` apart, so the payload is nobody else's, and a refusal that walked
/// away from it turned a reported gap into a leak. The release stands BEFORE
/// the throw, because there is no other path out of a block whose only
/// statement throws.
fn hole_in_a_side(what: &str, payload: &Read, t: &BodyTranslator) -> String {
    let throw = format!("{};\n", crate::body::hole_text(what));
    let Some(ty) = payload.ty.as_ref() else { return throw };
    let Some(types) = t.types.as_ref() else { return throw };
    let borrowed = types.borrow();
    if !crate::ownership::drops_of(&borrowed.probe(), ty).is_droppable() {
        return throw;
    }
    format!("dropOwned({});\n{}", payload.expr, throw)
}

/// What an arm owes if its GUARD throws: the payload the side already read.
///
/// `unwrap()` and `unwrapErr()` take the `Result` apart and hand the payload
/// over, so from the read onwards it belongs to whichever arm is being tried.
/// The arm's own `finally` covers its body; a guard is made before that
/// `finally` is entered, and a guard that panicked left the payload to nobody.
/// Nothing of the arm has run at that point, so the release is unconditional.
fn released_payload(name: &str, owed: &[crate::ownership::Owned]) -> String {
    let mut out = String::new();
    for value in owed.iter().rev() {
        if let Some(release) = value.drops.release(name) {
            out.push_str(&format!("{}\n", release));
        }
    }
    out
}

/// The same, for a refusal that never bound the payload to a name of its own.
fn payload_release(payload: &Read, t: &BodyTranslator) -> String {
    let Some(ty) = payload.ty.as_ref() else { return String::new() };
    let Some(types) = t.types.as_ref() else { return String::new() };
    let borrowed = types.borrow();
    match crate::ownership::drops_of(&borrowed.probe(), ty).release(&payload.expr) {
        Some(release) => format!("{}\n", release),
        None => String::new(),
    }
}

/// Does this inner pattern take a DROPPABLE name out of the payload?
fn takes_something_droppable(
    inner: &syn::Pat,
    arm: &syn::Arm,
    t: &BodyTranslator,
    scrutinee_ty: Option<&crate::ty::Ty>,
) -> bool {
    let _bindings = t.enter_pattern(&arm.pat, scrutinee_ty);
    let Some(types) = t.types.as_ref() else { return true };
    crate::body::pattern_names(inner).iter().any(|name| {
        let borrowed = types.borrow();
        match borrowed.lookup(name) {
            // A name the engine cannot type is one it cannot answer for.
            None => true,
            Some(ty) => crate::ownership::drops_of(&borrowed.probe(), &ty).is_droppable(),
        }
    })
}

/// What an arm's INNER pattern does to the payload the side already read.
enum Inner {
    /// `Err(e)`, `Ok(_)`, `_`: the side's own `isOk()` already decided the
    /// variant, and the pattern takes whatever is there.
    Whole,
    /// `Err(RetrievalError::NoDurablePeers)`, `Ok(None)`: a question about the
    /// payload and no name taken out of it.
    Tests(syn::Pat),
    /// `Ok(Some(state))`: a question, and a name for the WHOLE payload. The port
    /// writes `Option<T>` as `T | null`, so the inner name is the payload
    /// itself and there is no wrapper to release.
    TestsAndTakesAll { test: syn::Pat, name: String },
    /// `Err(SqlGenerationError::PlaceholderCountMismatch { expected, found })`:
    /// a question, and names for PART of the payload.
    TestsAndBindsPart(syn::Pat),
}

fn inner_test(arm: &syn::Arm) -> Inner {
    let syn::Pat::TupleStruct(ts) = &arm.pat else { return Inner::Whole };
    let Some(inner) = ts.elems.first() else { return Inner::Whole };
    if BodyTranslator::is_irrefutable(inner) {
        return Inner::Whole;
    }
    if BodyTranslator::binds_nothing(inner) {
        return Inner::Tests(inner.clone());
    }
    if let Some(name) = takes_the_whole_nullable(inner) {
        return Inner::TestsAndTakesAll { test: inner.clone(), name };
    }
    Inner::TestsAndBindsPart(inner.clone())
}

/// `Some(x)` with `x` a plain name: the one shape whose test leaves no wrapper,
/// because the port writes `Option<T>` as `T | null` and `x` IS the payload.
fn takes_the_whole_nullable(inner: &syn::Pat) -> Option<String> {
    let syn::Pat::TupleStruct(ts) = inner else { return None };
    if ts.path.segments.last()?.ident != "Some" || ts.elems.len() != 1 {
        return None;
    }
    let syn::Pat::Ident(ident) = ts.elems.first()? else { return None };
    if ident.subpat.is_some() {
        return None;
    }
    Some(crate::body::translate_pat(ts.elems.first()?))
}
