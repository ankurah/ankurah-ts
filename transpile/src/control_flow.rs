//! `if`, `if let` and the value a block's last expression produces.
//!
//! Nothing here carries drops any more. A block that owns something wraps its
//! body in `try`/`finally` (see `ownership`), and a `return` written in any
//! branch leaves through that `finally` — which is what Rust's drop glue does
//! at every early exit, and what threading a list of drop calls into each
//! branch could never cover for a `?`, a `break` or an unwind.

pub mod awaiting;
pub(crate) mod form;
mod let_chain;
pub mod sentinel;
#[cfg(test)]
pub(crate) mod sentinel_tests;

use crate::body::{indent, translate_pat, BodyTranslator};
use crate::match_expr;

/// Whether a branch produces the block's value or just runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Position {
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

/// Does this `loop` hand a value out through a `break`?
///
/// `loop { break }` is a statement whose value is `()`; `loop { break 9 }` is an
/// expression whose value is the payload, and in tail position that payload is
/// what the function answers.
fn breaks_with_a_value(loop_expr: &syn::ExprLoop) -> bool {
    struct Carried {
        found: bool,
    }
    impl syn::visit::Visit<'_> for Carried {
        fn visit_expr_break(&mut self, brk: &syn::ExprBreak) {
            if brk.expr.is_some() {
                self.found = true;
            }
        }
        // A nested loop's own `break` names that loop, not this one.
        fn visit_expr_loop(&mut self, _: &syn::ExprLoop) {}
        fn visit_expr_while(&mut self, _: &syn::ExprWhile) {}
        fn visit_expr_for_loop(&mut self, _: &syn::ExprForLoop) {}
        fn visit_expr_closure(&mut self, _: &syn::ExprClosure) {}
    }
    let mut carried = Carried { found: false };
    syn::visit::Visit::visit_block(&mut carried, &loop_expr.body);
    carried.found
}

pub fn translate_expr_in_return_position(expr: &syn::Expr, t: &BodyTranslator) -> String {
    match in_value_position(expr, t) {
        (value, Wrote::Value) => format!("return {};", value),
        (statements, Wrote::Statements) => statements,
    }
}

/// What a lowering wrote for an expression whose VALUE the position wants.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wrote {
    /// One expression: the text IS the value, and whoever asked for it writes
    /// the `return` or puts it where an expression goes.
    Value,
    /// A run of statements that has already done what the position wanted —
    /// the `return` is inside it, or it leaves without a value at all.
    Statements,
}

/// The same, told apart: the text, and which of the two forms it is.
///
/// F3/K2: the caller used to decide by reading the text back — a body holding
/// `";\n"` was called statements, so a nested `match` written as an if-chain
/// took no `return` and the arm answered `undefined`. The lowering knows which
/// form it wrote, so it says so.
pub fn in_value_position(expr: &syn::Expr, t: &BodyTranslator) -> (String, Wrote) {
    let statements = |text: String| (text, Wrote::Statements);
    match expr {
        syn::Expr::If(if_expr) => statements(translate_if_at(if_expr, t, Position::Returning)),
        syn::Expr::Match(match_expr) => statements(match_expr::translate_match_returning(match_expr, t)),
        // A block hands its value on from its tail, so what the position wants
        // of the block it wants of the tail — re-keyed onto the tail, because
        // an expectation is matched by the span of the expression it was
        // written for.
        syn::Expr::Block(block) => {
            let want = t.expectation_for(expr);
            if block.block.stmts.len() == 1 {
                if let syn::Stmt::Expr(inner, None) = &block.block.stmts[0] {
                    return t.expecting(inner, want.as_ref(), || in_value_position(inner, t));
                }
            }
            let body = match block.block.stmts.last() {
                Some(syn::Stmt::Expr(tail, None)) => {
                    t.expecting(tail, want.as_ref(), || t.translate_block(&block.block))
                }
                _ => t.translate_block(&block.block),
            };
            statements(format!("{{\n{}}}", indent(&body)))
        }
        // A `for` and a `while` are statements whose value is `()`; Rust gives
        // neither a `break` payload. A `loop` may have one — `loop { .. break 9 }`
        // — and in TAIL position that payload IS what the function answers.
        // Written as a statement it came out `break /* 9 */` and the function
        // fell off the end returning `undefined`.
        syn::Expr::Loop(loop_expr) if breaks_with_a_value(loop_expr) => {
            let (held, lifted) = t.with_own_hoists(|| t.expr_value(expr));
            statements(format!("{}return {};", crate::ownership::hoisted("", &lifted), held))
        }
        syn::Expr::ForLoop(_) | syn::Expr::While(_) | syn::Expr::Loop(_) => statements(t.expr(expr)),
        // These already leave the function, so putting a `return` in front of
        // one wrote `return return Result.Ok(..)`, which does not parse.
        syn::Expr::Return(_) | syn::Expr::Break(_) | syn::Expr::Continue(_) => {
            statements(format!("{};", t.expr(expr)))
        }
        _ => {
            // A block's last expression is its value, so a field read here
            // hands the field to whoever asked for the block.
            let ts = t.moved_value(expr);
            // A macro whose lowering is a run of statements has already said
            // what it does: `assert!(c)` is `if (!(c)) throw ..`, `bail!(..)`
            // is a `return`, and `panic!` and its family are a `throw`.
            // `return if (..) throw ..` does not parse. Answered from the macro
            // the source NAMED, not from the first word of the text.
            if let syn::Expr::Macro(mac) = expr {
                // H1: `select!` is the one whose answer the NAME cannot give —
                // its lowering writes one of two forms and records which.
                if crate::macros::items::writes_statements(&mac.mac.path)
                    || (crate::macros::items::is_select(&mac.mac.path)
                        && t.own.select_wrote_statements.get())
                {
                    return statements(ts);
                }
            }
            // A `throw` — `panic!`, `unreachable!`, and every other lowering
            // that writes one — is already a terminator.
            if ts.starts_with("throw ") {
                return statements(format!("{};", ts));
            }
            // A tail whose Rust value is `()` hands nothing back, and the port's
            // spelling of the same call may still produce something:
            // `Vec::push` answers `()` where `Array.prototype.push` answers the
            // new length, and returning that from a `void` function is a value
            // its signature does not admit. Asking is not translating, so the
            // question this resolution defers is not reported twice.
            let mark = t.mark();
            let unit = matches!(t.resolve_expr_type(expr), Ok(crate::ty::Ty::Unit));
            t.rewind(mark);
            if unit {
                return statements(format!("{};", ts));
            }
            (ts, Wrote::Value)
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
    // A `let` anywhere but the first conjunct: the chain nests, one `if` per
    // `let`, because that is where the binding it introduces is in scope.
    if let_chain::has_inner_let(&if_expr.cond) {
        return (String::new(), let_chain::translate(if_expr, t, position));
    }
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
pub(crate) fn else_part(
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
pub(crate) fn branch(block: &syn::Block, t: &BodyTranslator, position: Position) -> String {
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
                // Whatever this branch produces IS what the function answers,
                // so the return type is the branch's expectation — re-keyed
                // onto the branch's own span, because an expectation is matched
                // by the span of the expression it was written for. Live at
                // `core/indexing/encoding.rs`, where the `else` of an `if`
                // inside a match arm writes `Ok(bytes.into_iter().map(..)
                // .collect())` and the `collect` had nothing saying what it
                // built.
                let want = t.fn_return.clone();
                let (body, lifted) = t.with_own_hoists(|| {
                    t.expecting(expr, want.as_ref(), || {
                        translate_expr_in_return_position(expr, t)
                    })
                });
                // J3: what the branch lifted ABOVE its flag — an argument that
                // can throw before the call starts — stands ahead of it.
                let before = std::mem::take(&mut *t.own.before_flags.borrow_mut()).join("");
                return format!(
                    "{}{}{}",
                    before,
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
    let scrutinee_ty = t.borrowed_scrutinee_type(&let_expr.expr);
    let bound = t.enter_pattern(&let_expr.pat, scrutinee_ty.as_ref());
    // Where the pattern took a value out of the scrutinee, the branch owns it
    // and releases it however the branch is left.
    let owned = t.claim_bindings(&crate::body::pattern_names(&let_expr.pat), &then_branch.stmts);
    // Taking the payload out means the scrutinee stops being the block's to
    // release — otherwise the payload is dropped twice. Where the scrutinee is
    // a wrapper the port builds rather than the payload itself, that leaves the
    // wrapper for this construct to release: `Option<T>` is `T | null` and has
    // no wrapper, an enum is an object of its own and does.
    let wrapper = t.let_takes(let_expr) == crate::ownership::scrutinee::Takes::Payload
        && !t
            .scrutinee_type(&let_expr.expr)
            .is_some_and(|ty| t.is_nullable(&ty));
    let then_body = t.wrap_bindings(&owned, branch(then_branch, t, position));
    let guard_str = guard.map(|g| t.expr(g)).unwrap_or_default();
    drop(bound);

    let else_part = else_part(else_branch, t, position);

    let subject = t.fresh_temp();
    // The path where the pattern did not match took nothing out, so the value
    // this construct read is still whole and nobody else owns it: it is
    // released here, the way a `while let` releases the turn it did not take.
    // The path that *did* match is the reported one — the payload belongs to
    // the branch from there, and marking the wrapper moved is what `intoMatch`
    // does and an `if` cannot.
    let abandoned = match wrapper {
        true => t.release_of(&let_expr.expr, &subject).unwrap_or_default(),
        false => String::new(),
    };
    if wrapper {
        t.fallback(
            syn::spanned::Spanned::span(let_expr),
            "this pattern takes the payload out of the value it tests, and the wrapper it came \
             out of is not marked moved, so nothing releases the rest of it on the path that \
             matched",
        );
    }
    let else_part = if abandoned.is_empty() {
        else_part
    } else {
        // The release runs on the failed-pattern path whether or not the source
        // wrote an `else`.
        let rest = else_part
            .strip_prefix(" else {\n")
            .and_then(|s| s.strip_suffix("}"))
            .unwrap_or("");
        format!(" else {{\n{}{}}}", indent(&format!("{}\n", abandoned)), rest)
    };
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
        // The binding scope closed at `drop(bound)` above, so the borrowed-ness
        // of the value being taken apart is said again here: a borrowed
        // `Result` is READ, not unwrapped.
        _ => t.matching(scrutinee_ty.as_ref(), || t.pattern_test(&subject, &let_expr.pat)),
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
