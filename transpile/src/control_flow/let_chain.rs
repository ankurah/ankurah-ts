//! `if cond && let PAT = e && ..` — Rust 2024's let-chains.
//!
//! A `let` inside a condition binds for the rest of that condition and for the
//! branch it guards. JavaScript has nowhere to put such a binding inside a `&&`,
//! so each `let` opens a nested `if` and every conjunct after it is written
//! inside — which is what "binds for the rest of the condition" means.
//!
//! Before this, `Expr::Let` in a condition was written as `/* let .. */`, a
//! comment where an operand belongs: `a && /* let .. */ && b` does not parse.
//! `if_chain` already handled the one shape where the `let` is the leftmost
//! conjunct; `&&` is left-associative, so `let a = .. && let b = .. && c && d`
//! nests the lets three levels down on the left and that check never saw them.

use super::{Position, branch};
use crate::body::{BodyTranslator, indent};

/// `a && b && c` as `[a, b, c]`. Anything that is not an `&&` is one conjunct.
pub fn conjuncts(expr: &syn::Expr) -> Vec<&syn::Expr> {
    match expr {
        syn::Expr::Binary(bin) if matches!(bin.op, syn::BinOp::And(_)) => {
            let mut out = conjuncts(&bin.left);
            out.extend(conjuncts(&bin.right));
            out
        }
        syn::Expr::Paren(p) => conjuncts(&p.expr),
        other => vec![other],
    }
}

/// Does this condition put a `let` anywhere but its first conjunct? That is the
/// shape `if_chain`'s own let-handling does not reach.
pub fn has_inner_let(cond: &syn::Expr) -> bool {
    let parts = conjuncts(cond);
    parts.len() > 1 && parts.iter().skip(1).any(|e| matches!(e, syn::Expr::Let(_)))
}

/// Write the whole chain as nested `if`s.
pub fn translate(
    if_expr: &syn::ExprIf,
    t: &BodyTranslator,
    position: Position,
) -> String {
    let parts = conjuncts(&if_expr.cond);
    let else_text = super::else_part(&if_expr.else_branch, t, position);
    if if_expr.else_branch.is_some() {
        // Every level of the nest needs the same `else`, so its text is written
        // once per level. It is a branch, not an expression, so nothing is
        // evaluated twice — but the output grows, and a reader should know why.
        t.fallback(
            syn::spanned::Spanned::span(&if_expr.cond),
            "this let-chain has an `else`, and each `let` in the chain opens a nested `if`, so \
             the `else` branch is written once per level",
        );
    }
    // The scopes the pattern bindings live in have to stay open while the parts
    // to their right are written, and close in reverse — so the whole chain is
    // built on the way down and assembled on the way back up.
    build(&parts, if_expr, t, position, &else_text)
}

fn build(
    parts: &[&syn::Expr],
    if_expr: &syn::ExprIf,
    t: &BodyTranslator,
    position: Position,
    else_text: &str,
) -> String {
    let Some((first, rest)) = parts.split_first() else {
        return branch(&if_expr.then_branch, t, position);
    };
    match first {
        syn::Expr::Let(let_expr) => {
            let scrutinee = t.expr(&let_expr.expr);
            let scrutinee_ty = t.borrowed_scrutinee_type(&let_expr.expr);
            let subject = t.fresh_temp();
            let _bound = t.enter_pattern(&let_expr.pat, scrutinee_ty.as_ref());
            let (test, bind) = t.pattern_test(&subject, &let_expr.pat);
            let inner = build(rest, if_expr, t, position, else_text);
            drop(_bound);
            format!(
                "{{\n  const {} = {};\n  if ({}) {{\n{}  }}{}\n}}",
                subject,
                scrutinee,
                test,
                indent(&indent(&format!("{}{}", bind, inner))),
                else_text
            )
        }
        // A plain conjunct is a test, and everything to its right is inside it,
        // because Rust stops at the first false.
        other => {
            let (cond, lifted) = t.with_own_hoists(|| t.expr(other));
            let (cond, before) = t.settle_condition(cond, &lifted);
            let inner = build(rest, if_expr, t, position, else_text);
            format!(
                "{}if ({}) {{\n{}}}{}",
                before,
                cond,
                indent(&inner),
                else_text
            )
        }
    }
}
