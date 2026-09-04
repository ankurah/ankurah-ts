//! `if`, `if let` and the value a block's last expression produces.
//!
//! Nothing here carries drops any more. A block that owns something wraps its
//! body in `try`/`finally` (see `ownership`), and a `return` written in any
//! branch leaves through that `finally` — which is what Rust's drop glue does
//! at every early exit, and what threading a list of drop calls into each
//! branch could never cover for a `?`, a `break` or an unwind.

use crate::body::{indent, translate_pat, BodyTranslator};
use crate::match_expr;

/// Whether a branch produces the block's value or just runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Position {
    /// The `if` is a statement: its branches run and produce nothing.
    Statement,
    /// The `if` is the value of the function or of the block around it, so each
    /// branch ends in a `return`.
    Returning,
}

/// Translate an if expression (handles if-let patterns)
pub fn translate_if(if_expr: &syn::ExprIf, t: &BodyTranslator) -> String {
    translate_if_at(if_expr, t, Position::Statement)
}

/// The same `if`, where each branch produces the enclosing block's value.
pub fn translate_expr_in_return_position(expr: &syn::Expr, t: &BodyTranslator) -> String {
    match expr {
        syn::Expr::If(if_expr) => translate_if_at(if_expr, t, Position::Returning),
        syn::Expr::Match(match_expr) => match_expr::translate_match_returning(match_expr, t),
        syn::Expr::Block(block) => {
            if block.block.stmts.len() == 1 {
                if let syn::Stmt::Expr(inner, None) = &block.block.stmts[0] {
                    return translate_expr_in_return_position(inner, t);
                }
            }
            let body = t.translate_block(&block.block);
            format!("{{\n{}}}", indent(&body))
        }
        // A loop is a statement and its value is `()`.
        syn::Expr::ForLoop(_) | syn::Expr::While(_) | syn::Expr::Loop(_) => t.expr(expr),
        _ => {
            // A block's last expression is its value, so a field read here
            // hands the field to whoever asked for the block.
            let ts = t.moved_value(expr);
            // throw/panic is already a terminator — don't prefix with return
            if ts.starts_with("throw ") {
                format!("{};", ts)
            } else {
                format!("return {};", ts)
            }
        }
    }
}

fn translate_if_at(if_expr: &syn::ExprIf, t: &BodyTranslator, position: Position) -> String {
    let (before, chain) = if_chain(if_expr, t, position);
    format!("{}{}", before, chain)
}

/// The `if` itself, handed back apart from the statements its condition needs
/// standing above it.
///
/// The two are separated so that an `else if` can put those statements inside a
/// block of its own. Rust evaluates each condition in the chain only once the
/// one above it has failed, and a condition that lifted a temporary is a run of
/// statements rather than an expression — written between the `else` and the
/// `if` it belongs to, that run is not a program at all.
fn if_chain(if_expr: &syn::ExprIf, t: &BodyTranslator, position: Position) -> (String, String) {
    if let syn::Expr::Let(let_expr) = &*if_expr.cond {
        let written =
            translate_if_let(let_expr, &if_expr.then_branch, &if_expr.else_branch, None, t, position);
        return (String::new(), written);
    }

    // Handle let-chains: if let Some(x) = expr && guard { ... }
    if let syn::Expr::Binary(bin) = &*if_expr.cond {
        if matches!(bin.op, syn::BinOp::And(_)) {
            if let syn::Expr::Let(let_expr) = &*bin.left {
                let written = translate_if_let(
                    let_expr,
                    &if_expr.then_branch,
                    &if_expr.else_branch,
                    Some(&bin.right),
                    t,
                    position,
                );
                return (String::new(), written);
            }
        }
    }

    // A condition is its own temporary scope in Rust: what it produced is
    // dropped before the body runs. Leaving the temporaries where the statement
    // machinery put them held a lock for the whole body, so a body that dropped
    // the container it locked hit the outstanding-guard fatal.
    let (cond, lifted) = t.with_own_hoists(|| t.expr(&if_expr.cond));
    let (cond, before) = t.settle_condition(cond, &lifted);
    let then_body = branch(&if_expr.then_branch, t, position);
    let chain = format!(
        "if ({}) {{\n{}}}{}",
        cond,
        indent(&then_body),
        else_part(&if_expr.else_branch, t, position)
    );
    (before, chain)
}

/// What follows the `then` branch, from `else if` down to nothing at all.
fn else_part(
    else_branch: &Option<(syn::token::Else, Box<syn::Expr>)>,
    t: &BodyTranslator,
    position: Position,
) -> String {
    let Some((_, else_expr)) = else_branch else {
        return String::new();
    };
    match else_expr.as_ref() {
        syn::Expr::If(else_if) => {
            let (before, chain) = if_chain(else_if, t, position);
            if before.is_empty() {
                return format!(" else {}", chain);
            }
            // This condition takes a temporary of its own, and the statements
            // that take and release it run only on the path the `else` reaches.
            format!(" else {{\n{}}}", indent(&format!("{}{}\n", before, chain)))
        }
        syn::Expr::Block(block) => {
            format!(" else {{\n{}}}", indent(&branch(&block.block, t, position)))
        }
        _ => format!(" else {{\n{}}}", indent(&t.expr(else_expr))),
    }
}

/// One branch of an `if`. In returning position its last expression becomes the
/// value; otherwise `translate_block` writes it as it stands.
fn branch(block: &syn::Block, t: &BodyTranslator, position: Position) -> String {
    match position {
        Position::Statement => t.translate_block(block),
        Position::Returning => {
            if let (1, Some(syn::Stmt::Expr(expr, None))) = (block.stmts.len(), block.stmts.last())
            {
                // A one-expression branch is not written through the block
                // machinery, so it sets its own drop flags and keeps whatever it
                // lifted out of itself. Without the flags a branch that hands a
                // local away in tail position left the flag false and the
                // `finally` released a value somebody else owned; without the
                // hoists the declaration stood outside the branch that needed
                // it and ran on the path that did not.
                let flags = t.flag_sets_for(expr);
                let (body, lifted) =
                    t.with_own_hoists(|| translate_expr_in_return_position(expr, t));
                return format!(
                    "{}{}",
                    flags,
                    crate::ownership::hoisted(&format!("{}\n", body), &lifted)
                );
            }
            t.translate_block(block)
        }
    }
}

/// The name an `Ok(x)` pattern binds, where the pattern is one.
fn ok_binding(pat: &syn::Pat) -> Option<String> {
    let syn::Pat::TupleStruct(ts) = pat else {
        return None;
    };
    if ts.path.segments.last()?.ident != "Ok" {
        return None;
    }
    Some(ts.elems.first().map(translate_pat).unwrap_or_else(|| "v".to_string()))
}

/// Translate `if let PAT = e { .. } else { .. }`.
///
/// The scrutinee is read once, into a temporary, and both the test and the
/// binding are written against that. Writing the expression twice called it
/// twice: `if let Some(ordering) = comparison.step().await?` stepped the
/// comparison to make the test and stepped it again to bind the result, which
/// is a different program from the one Rust was given.
fn translate_if_let(
    let_expr: &syn::ExprLet,
    then_branch: &syn::Block,
    else_branch: &Option<(syn::token::Else, Box<syn::Expr>)>,
    guard: Option<&syn::Expr>,
    t: &BodyTranslator,
    position: Position,
) -> String {
    let scrutinee = t.expr(&let_expr.expr);
    // The names the pattern introduces are in scope for the branch it guards,
    // and for the guard expression written after it.
    let scrutinee_ty = t.scrutinee_type(&let_expr.expr);
    let bound = t.enter_pattern(&let_expr.pat, scrutinee_ty.as_ref());
    // Where the pattern took a value out of the scrutinee, the branch owns it
    // and releases it however the branch is left.
    let owned = t.claim_bindings(&crate::body::pattern_names(&let_expr.pat), &then_branch.stmts);
    let then_body = t.wrap_bindings(&owned, branch(then_branch, t, position));
    let guard_str = guard.map(|g| t.expr(g)).unwrap_or_default();
    drop(bound);

    let else_part = else_part(else_branch, t, position);

    let subject = t.fresh_temp();
    // `if let Ok(guard) = lock.lock()`. The port's `lock()` and `read()` hand
    // back the guard itself, so there is no `Ok` to test. Only those calls get
    // this: a `Result` that merely carries a `PoisonError` is a real `Result`
    // and keeps its test. The guard expression and the `else` branch stay —
    // the `else` is the poisoned-lock arm, which this runtime cannot reach, and
    // deleting it silently changed what the source said.
    let (test, bind) = match ok_binding(&let_expr.pat) {
        Some(var) if t.writes_the_value_not_the_result(&let_expr.expr) => {
            ("true".to_string(), format!("const {} = {};\n", var, subject))
        }
        _ => t.pattern_test(&subject, &let_expr.pat),
    };
    // A guard is written after the pattern's names, because it reads them.
    let body = if guard_str.is_empty() {
        format!("{}{}", bind, then_body)
    } else {
        format!("{}if ({}) {{\n{}}}\n", bind, guard_str, indent(&then_body))
    };
    // The temporary is scoped to the statement, so an `if let` beside another
    // does not see the first one's subject.
    format!(
        "{{\n  const {} = {};\n  if ({}) {{\n{}  }}{}\n}}",
        subject,
        scrutinee,
        test,
        indent(&indent(&body)),
        else_part
    )
}
