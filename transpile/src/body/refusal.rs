//! What a statement still owns when its own lowering refused.
//!
//! For: a refusal is discovered while the statement is being written, not
//! before it, and by then part of the statement may already stand in the
//! output. A `?` operand to the left of the refusal has been evaluated, its
//! temporary holds whatever it took, and the sequence the refused call was
//! walking was never taken at all — so the hole threw with two Tokens and a
//! Result owned by nobody. The block used to read a global hole count after
//! rendering and release the statement's source values ABOVE it, which is right
//! only where nothing of the statement ran.
//!
//! So the fact travels with the lowering: each hoist records what it declared
//! and whether its own lowering wrote a hole (I4).

use crate::body::BodyTranslator;
use crate::ownership;

/// The `?` hoist: lower the operand, record what it declared and whether it
/// refused, and answer the value the expression stands for.
pub(crate) fn hoist_a_try(
    t: &BodyTranslator,
    try_expr: &syn::ExprTry,
    want: Option<crate::ty::Ty>,
) -> String {
    // A hole in this hoist's declaration is where the throw stands: everything
    // lifted before it ran, nothing after it did, and the `unwrap` that would
    // have consumed this wrapper stands in the statement's own text, which is
    // never reached — and that is true only where the operand's OWN value is
    // the hole, which is what `try_operand` answers by leaving `temp` and
    // `wrapper` empty. Asked of the global counter (R1), a refusal buried in a
    // closure the operand passes made the whole statement take the refusal
    // path.
    let lowered = t.expecting(&try_expr.expr, want.as_ref(), || t.lower_try(try_expr));
    // R0(3): "the `unwrap` that follows consumes the wrapper, so it owes
    // nothing" is true only where that `unwrap` is REACHED. A second `?` in the
    // same statement leaves through its own `return` — and `values.next()`
    // three frames down throws — with the first `?`'s wrapper still holding its
    // `Ok` payload and nobody to release it. Rust drops that temporary on both
    // paths: the `?` return is a scope exit, and an unwind drops what the scope
    // holds. So the wrapper is released however the statement is left, asked of
    // the runtime first, because the `unwrap` marks it moved on the path that
    // did reach it. Fifty-one of ankql's leak reports were this one fact.
    //
    // Only where there IS a wrapper. A `?` on an `Option` writes no wrapper —
    // the temporary is the payload itself, an arbitrary value that may be a
    // plain array or `Map` carrying no move mark — and releasing that from a
    // guard is S1's double drop. That half waits for the lexical flag.
    let wrapper = lowered.wrapper.is_some();
    // W14: an `Option` `?` writes no wrapper, so the temporary IS the payload,
    // and a payload with drop glue is released from a flag this frame declares
    // — never from the runtime guard, which an array, a `Map` or a `Set`
    // answers "nobody has taken it" to whatever happened (S1). Where the text
    // cannot leave before reading it, `hoisted` writes neither the flag nor the
    // release.
    let payload = !wrapper && lowered.temp.is_some() && t.payload_owes_a_release(&try_expr.expr);
    // Named from the temporary rather than from the shared counter, because the
    // flag is written only where the text can leave before the payload is read
    // — a decision `hoisted` makes later — and a number taken here and then not
    // written renumbers every hoist below it in the emitted file.
    let flag = payload.then(|| format!("{}_kept", lowered.temp.as_deref().unwrap_or("_r")));
    let declaration = match &flag {
        Some(flag) => format!("let {} = false;\n{}", flag, lowered.declaration),
        None => lowered.declaration.clone(),
    };
    // U3: a local this operand hands away has its move flag set here, right
    // above the call that hands it away, rather than above the statement's
    // whole prelude — which is above the arguments that were lifted so the flag
    // could stand below them. A flag a hoist nested INSIDE this one already
    // claimed stays there: that hoist runs first and is nearer the transfer.
    let sets = claimed_here(t, &try_expr.expr);
    t.own.prelude.borrow_mut().push(ownership::Hoist {
        declaration,
        owned: None,
        temp: lowered.temp.clone(),
        refused: lowered.temp.is_none() && lowered.wrapper.is_none(),
        released_if_unreached: false,
        wrapper,
        sets,
        payload,
        // GG5/FF8: every `await` this statement writes BELOW the hoist runs
        // while the wrapper is in hand; one written inside the `?`'s own
        // operand runs in the hoist's declaration, before the wrapper exists.
        suspends: crate::ownership::hoisting::suspends_before(
            &t.own.statement_awaits.borrow(),
            &try_expr.expr,
        ),
        droppable: false,
        flag,
    });
    lowered.value
}

/// The move-flag assignments this `?` operand owes and no hoist already in the
/// prelude has taken.
fn claimed_here(t: &BodyTranslator, operand: &syn::Expr) -> String {
    t.flag_sets_for(operand)
        .lines()
        .filter(|line| !t.own.prelude.borrow().iter().any(|h| h.sets.lines().any(|s| s == *line)))
        .map(|line| format!("{}\n", line))
        .collect()
}

/// One statement of a block, written for the path where one of its HOISTS
/// refused: the temporaries the prefix produced and the source values it named
/// and did not consume are released however the statement is left.
///
/// `rest` is what follows the statement in the block. After a refused `let` it
/// goes INSIDE the `try` this writes, because a `let` puts a name in scope for
/// everything after it and a `try` is a scope: outside it, the rest of the block
/// names something nothing declares. It is never reached — the hole above it
/// throws — but it still has to be code a JavaScript engine will read.
pub(crate) fn statement_that_refused(
    t: &BodyTranslator,
    stmt: &syn::Stmt,
    text: String,
    rest: String,
    below: &[syn::Stmt],
    prelude: &[ownership::Hoist],
    dispositions: &ownership::Dispositions,
    ordinals: &std::cell::RefCell<std::collections::HashMap<String, usize>>,
) -> String {
    // What this statement still owns, less whatever a call that finished before
    // the hole has already taken.
    let rendered: String = prelude
        .iter()
        .map(|h| h.declaration.as_str())
        .chain(std::iter::once(text.as_str()))
        .collect();
    let owed = t.released_after_a_refusal(stmt, below, dispositions, ordinals);
    let mut prelude: Vec<ownership::Hoist> = prelude.to_vec();
    // A temporary a completed call took before the hole is that call's now, so
    // it owes nothing here. The question is asked of what stands AFTER the
    // hoist's own declaration: the declaration itself is where the temporary is
    // produced, and `const _r0 = pass(first);` is not `pass` taking `_r0`.
    for i in 0..prelude.len() {
        let below: String = prelude[i + 1..]
            .iter()
            .map(|h| h.declaration.as_str())
            .chain(std::iter::once(text.as_str()))
            .collect();
        // W13: the hole is in THIS hoist's own declaration, which is where
        // the throw stands; nothing below it ran.
        let refused_here = crate::body::holds_a_hole(&prelude[i].declaration);
        if let Some(temp) = prelude[i].temp.clone() {
            if !refused_here && handed_over_before_the_hole(&below, &temp) {
                prelude[i].temp = None;
            }
        }
    }
    let (inner, after_it) = match matches!(stmt, syn::Stmt::Local(_)) {
        true => (format!("{}{}", text, rest), String::new()),
        false => (text, rest),
    };
    // S1: each release carries a flag this frame declares, and the flag is set
    // where the transfer is WRITTEN — immediately after the hoist whose
    // declaration performs it, or above the statement's own text when the
    // transfer is there. `(hole()?, pass_vec(rest)?)` sets nothing, because the
    // hoist that would consume `rest` is never reached; `(pass_vec(rest)?,
    // hole()?)` sets it, because that hoist ran. Reading the value's own
    // `isMoved` answered "nobody has taken it" for every array, `Map` and `Set`
    // in the port and dropped the contents twice.
    // HH1: a name the FRAME already carries a flag for is not in `owed` — its
    // release is written by the frame, under that flag — but the flag still has
    // to be SET where the transfer is, and a refused statement never reaches
    // the ordinary `flag_sets`. `(count(rest)?, hole()?)` declared a flag for
    // `rest`, set it nowhere, and the frame's guard was then dropped as a flag
    // nothing sets: `dropOwned(rest)` ran on top of the `count` that had taken
    // it.
    for site in ownership::Scan::new(t).shallow(stmt) {
        let Some(flag) = t.flag_for(&site.name) else { continue };
        if owed.iter().any(|release| release.name == site.name) {
            continue;
        }
        let Some(hoist) = prelude
            .iter_mut()
            .find(|hoist| mentions(&hoist.declaration, &site.name))
        else {
            continue;
        };
        if crate::body::holds_a_hole(&hoist.declaration) {
            continue;
        }
        let at = hoist.declaration.find('\n').map_or(hoist.declaration.len(), |i| i + 1);
        let set = format!("{} = true;\n", flag);
        if !hoist.declaration.contains(&set) {
            hoist.declaration.insert_str(at, &set);
        }
    }
    let mut declarations = String::new();
    let mut after = String::new();
    for release in &owed {
        match prelude
            .iter_mut()
            .find(|hoist| mentions(&hoist.declaration, &release.name))
        {
            // The set goes after the hoist's FIRST line, which is the one that
            // calls. A `?` hoist's second line is its early exit, and a set
            // written below that would be skipped on the path the exit takes —
            // leaving the `finally` to release what the call on the line above
            // consumed.
            Some(hoist) => {
                let at = hoist.declaration.find('\n').map_or(hoist.declaration.len(), |i| i + 1);
                hoist.declaration.insert_str(at, &release.set());
                declarations.push_str(&release.declaration());
                after.push_str(&release.guarded());
            }
            // No hoist performs this transfer, so the call that would have
            // taken the value stands in the statement's own TEXT. There is no
            // point in the emitted output where a flag could be set that the
            // hole does not abort — one written here would be a `let` nothing
            // assigns and a test that is always false (E15) — so the text
            // itself is asked instead, and the release stands unguarded where
            // the text says nothing took the value.
            None if !handed_over_before_the_hole(&rendered, &release.name) => {
                after.push_str(&format!("{}\n", release.release))
            }
            None => {}
        }
    }
    format!(
        "{}{}{}",
        declarations,
        ownership::hoisted_when_refused(&inner, &prelude, &after),
        after_it
    )
}

/// Did a call take this value before the hole threw?
///
/// A refused statement owes a release for what it named and did not hand away,
/// and this is the second half of that. Everything that RAN before the throw
/// has taken what it takes: `r.unwrapErr()` standing above a refusing arm, and
/// `o.intoMatch({ W: (v) => <hole> })`, whose arrow the call itself invoked. A
/// name that only stands as an ARGUMENT to a call the hole aborts —
/// `take2(held, <hole>)` — is still this frame's.
///
/// W4, RECORDED AND NOT FIXED: this is read off the emitted TEXT — "is there a
/// `)` after the name and before the first `unsupported(`" — and a call that
/// merely BORROWED the name answers yes (`size(&held)` puts a `)` after
/// `held`), as does an unrelated call standing beside it (`take2(held, side(),
/// <hole>)`, where `side()`'s own paren supplies the evidence). A borrow is
/// indistinguishable from a move in rendered text.
///
/// What replaces it has to know THREE things, and a log of "which calls were
/// handed which names by value" only knows one. It must also know that the
/// call's own text SURVIVED — `held.into_iter()` in
/// `held.into_iter().map(..).collect()` is written and then thrown away when
/// the `collect` refuses, so nothing took `held` — and it must include the
/// consumptions the port writes OUTSIDE the call lowering, of which
/// `expr.intoMatch({ .. })` is one: its subject is consumed by the match
/// writer, and a log that misses it releases a value the arms already own.
///
/// Answering yes wrongly leaks, which the collector reports; answering no
/// wrongly drops a value the callee owns, which is fatal. This still leans to
/// the report: a call is recorded as having taken every name it was handed in a
/// by-value position, whether or not the callee kept it.
fn handed_over_before_the_hole(rendered: &str, name: &str) -> bool {
    let Some(hole) = crate::body::holes::hole_at(rendered) else { return false };
    let before = &rendered[..hole];
    let mut from = 0;
    while let Some(at) = mentions_at(&before[from..], name) {
        let at = from + at;
        if before[at + name.len()..].contains(')') {
            return true;
        }
        from = at + name.len();
    }
    false
}

/// Where this emitted text names `what` as a whole identifier, if it does.
pub(crate) fn mentions_at(text: &str, what: &str) -> Option<usize> {
    let is_part = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    let mut from = 0;
    while let Some(at) = text[from..].find(what) {
        let at = from + at;
        let before = text[..at].chars().next_back().is_some_and(is_part);
        let after = text[at + what.len()..].chars().next().is_some_and(is_part);
        if !before && !after {
            return Some(at);
        }
        from = at + what.len();
    }
    None
}

/// Does this emitted text name `what` as a whole identifier?
///
/// Substring alone said `rest` of `restore(x)`, and the flag would then have
/// been set by a hoist that consumed nothing.
pub(crate) fn mentions(text: &str, what: &str) -> bool {
    mentions_at(text, what).is_some()
}

/// The operand of a `?`, or the whole `?` written out where that operand
/// REFUSED.
///
/// A hole has no wrapper to test: it throws where the operand stood. Written
/// with the test around it, the port emitted `if (_r0.isErr())` over a value
/// typed `never` — which no type checker accepts, and which reads as though the
/// hole might have answered something. So the declaration is the hole and
/// nothing else, and the `?` stands for the name it left behind.
pub(crate) fn try_operand(
    t: &BodyTranslator,
    operand: &syn::Expr,
) -> Result<String, crate::body::Lowered> {
    let before = crate::body::holes_written();
    let inner = t.expr(operand);
    // R1: the question is whether THIS OPERAND has a value to test, so it is
    // asked of the operand's own value. Asked of the global hole counter it
    // answered yes to a refusal anywhere in the subtree — inside a closure the
    // operand passes, in one branch of an `if`, in an argument beside it — and
    // `a.or_else(|| xs.into_iter().next())?` lost `if (_r0 == null) return
    // null;` and computed on null where Rust answers `None`.
    if !crate::body::lowered_a_hole(before, &inner) {
        return Ok(inner);
    }
    let temp = t.fresh_hoist("_r");
    Err(crate::body::Lowered {
        declaration: format!("const {} = {};\n", temp, inner),
        value: temp,
        wrapper: None,
        temp: None,
    })
}

impl BodyTranslator<'_> {
    /// Does what a `?` on an `Option` hands back owe a release?
    ///
    /// The operand is an `Option<T>` and the payload is its `T`: `Option<Vec<
    /// Token>>` hands back an array of values with drop glue, and a `?` that
    /// leaves through a LATER one still holds it.
    pub(crate) fn payload_owes_a_release(&self, operand: &syn::Expr) -> bool {
        let Some(tc) = &self.types else { return false };
        let Ok(ty) = self.quietly(|| self.resolve_expr_type(operand)) else { return false };
        let crate::ty::Ty::Named { args, .. } = ty.peel_refs() else { return false };
        let Some(payload) = args.first() else { return false };
        crate::ownership::drops_of(&tc.borrow().probe(), payload).is_droppable()
    }
}

impl BodyTranslator<'_> {
    /// A write to a place the runtime hands out only as a VALUE, refused.
    ///
    /// A holder reaches what it carries through an accessor that READS it, so
    /// an atomic's `store` or `fetch_add` through one would land on a copy and
    /// be lost. The hole says so where the call stands.
    pub(crate) fn lost_write_through_a_holder(
        &self,
        call: &syn::ExprMethodCall,
        found: &crate::registry::MethodResolution,
        rust_method: &str,
    ) -> Option<crate::native_types::MethodTranslation> {
        let tc = self.types.as_ref()?;
        if found.accessors().is_empty() {
            return None;
        }
        if !crate::native_types::writes_through_the_holder(
            tc.borrow().registry,
            rust_method,
            found.receiver_type(),
        ) {
            return None;
        }
        let holder = self
            .quietly(|| self.resolve_expr_type(&call.receiver))
            .unwrap_or(crate::ty::Ty::Infer);
        let message = format!(
            "`{}` WRITES what the `{}` holds, and it is reached through an accessor that hands \
             out the value rather than the place",
            rust_method,
            tc.borrow().registry.describe(&holder)
        );
        Some(crate::native_types::MethodTranslation::Refused {
            fallback: Box::new(crate::native_types::MethodTranslation::Expr(
                crate::body::hole_text(&message),
            )),
            message,
        })
    }
}
