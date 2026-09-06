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
    // never reached.
    let holes_before = crate::body::holes_written();
    let lowered = t.expecting(&try_expr.expr, want.as_ref(), || t.lower_try(try_expr));
    t.own.prelude.borrow_mut().push(ownership::Hoist {
        declaration: lowered.declaration,
        owned: None,
        temp: lowered.temp.clone(),
        refused: crate::body::holes_written() > holes_before,
    });
    lowered.value
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
    let after = t.released_after_a_refusal(stmt, dispositions, ordinals);
    let (inner, after_it) = match matches!(stmt, syn::Stmt::Local(_)) {
        true => (format!("{}{}", text, rest), String::new()),
        false => (text, rest),
    };
    format!("{}{}", ownership::hoisted_when_refused(&inner, prelude, &after), after_it)
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
    let holes_before = crate::body::holes_written();
    let inner = t.expr(operand);
    if crate::body::holes_written() == holes_before {
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
