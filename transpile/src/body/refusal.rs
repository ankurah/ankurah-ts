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
    // U3: a local this operand hands away has its move flag set here, right
    // above the call that hands it away, rather than above the statement's
    // whole prelude — which is above the arguments that were lifted so the flag
    // could stand below them. A flag a hoist nested INSIDE this one already
    // claimed stays there: that hoist runs first and is nearer the transfer.
    let sets = claimed_here(t, &try_expr.expr);
    t.own.prelude.borrow_mut().push(ownership::Hoist {
        declaration: lowered.declaration,
        owned: None,
        temp: lowered.temp.clone(),
        refused: lowered.temp.is_none() && lowered.wrapper.is_none(),
        released_if_unreached: false,
        wrapper,
        sets,
        droppable: false,
        flag: None,
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
    let owed = t.released_after_a_refusal(stmt, dispositions, ordinals);
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
/// and the second half of that is decided by the emitted text. Everything
/// textually complete before the first hole RAN, so a call that has both its
/// name and its closing paren before the hole has taken what it takes:
/// `r.unwrapErr()` standing above a refusing arm, and `o.intoMatch({ W: (v) =>
/// <hole> })`, whose arrow the call itself invoked. A name that only stands as
/// an ARGUMENT to a call the hole aborts — `take2(held, <hole>)` — has no `)`
/// between it and the hole, and is still this frame's.
///
/// Answering yes wrongly leaks, which the collector reports; answering no
/// wrongly drops a value the callee owns, which is fatal. This leans to the
/// report.
fn handed_over_before_the_hole(rendered: &str, name: &str) -> bool {
    let Some(hole) = crate::body::hole_at(rendered) else { return false };
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
