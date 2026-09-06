//! What the lowering WROTE for an expression: a run of statements or one
//! value, and whether every path out of it leaves.
//!
//! For: three questions used to be answered by reading the rendered text back.
//! `is_statements` called a body a run of statements when it contained `";\n"`
//! — which an expression written over several lines does — or when it started
//! with `if `. `begins_a_statement` guessed the same thing from the first word.
//! `leaves_the_arm` read the text backwards for a `return`, a `throw`, a
//! `break` or a `continue` on the last line that was not a closing brace.
//!
//! Each of the three was wrong about a shape the corpus writes:
//!
//! - `Expr::Placeholder => match values.next() { Some(v) => Ok(..), None =>
//!   Err(..) }` — a nested match as an arm's VALUE — renders as `if (…) { … }
//!   else { … }`, so `is_statements` said the arm had already done its work and
//!   no `return` went in front of either branch. `ankql/ast.ts`'s
//!   `Expr.populateRecursive` answered `undefined` from a method whose type is
//!   `Result<Expr, ParseError>`.
//! - `W::Light(n) if *n > 3 => { if *n > 100 { return 1; } }` ends in a
//!   CONDITIONAL jump. `leaves_the_arm` read the last line, found `return 1;`,
//!   and said the arm leaves — so the chain wrote no `break` after it and the
//!   arm below ran too, pushing 0 for `Light(5)`.
//!
//! So the answers are computed from the lowering's own inputs — the Rust
//! expression and the position it was written for — and carried beside the
//! text.

use crate::body::BodyTranslator;

/// Does the lowering write this expression as a run of STATEMENTS, rather than
/// as one expression whose value the position may take?
///
/// A `match` and a `select!` are the two shapes whose form is not settled by
/// the shape alone: the runtime's keyed `.match({..})` is a value and every
/// other strategy is an if-chain, and a `select!` that produces a value is one
/// expression while an escaping one opens `const _bN = [`. Each lowering
/// records which it took, so this must be asked straight after the expression
/// was written — the same call, the same `t`.
pub(crate) fn writes_statements(expr: &syn::Expr, t: &BodyTranslator) -> bool {
    match expr {
        syn::Expr::Block(_)
        | syn::Expr::If(_)
        | syn::Expr::ForLoop(_)
        | syn::Expr::While(_)
        | syn::Expr::Loop(_)
        | syn::Expr::Unsafe(_)
        | syn::Expr::TryBlock(_)
        | syn::Expr::Return(_)
        | syn::Expr::Break(_)
        | syn::Expr::Continue(_) => true,
        syn::Expr::Match(_) => t.last_match_wrote_statements.get(),
        syn::Expr::Macro(mac) => crate::macros::items::writes_statements(&mac.mac.path)
            || (crate::macros::items::is_select(&mac.mac.path)
                && t.own.select_wrote_statements.get()),
        syn::Expr::Group(group) => writes_statements(&group.expr, t),
        syn::Expr::Paren(paren) => writes_statements(&paren.expr, t),
        _ => false,
    }
}

/// Does the text the RETURN position wrote leave on every path?
///
/// That position writes a `return` wherever the expression hands a value back,
/// and it writes it RECURSIVELY: each branch of an `if`, each arm of a match
/// and a block's tail are all written for the same position. So the answer
/// follows the same shape the writing did, and the only text that runs on is
/// one whose Rust value is `()` — a block with no tail, a `for`, a call the
/// port spells as a statement.
///
/// The type question is asked exactly as `in_value_position` asks it, so the
/// two cannot disagree: that position writes `return <value>;` unless the type
/// is DEFINITELY `()`, and this says the text leaves under the same condition.
/// Answering it the other way round wrote a jump nothing reaches after three
/// arms of `Value::cast_to` and `SqliteStorageEngine`.
///
/// Asking is not translating, so what this resolution cannot say is not
/// reported here.
pub(crate) fn leaves_in_return_position(expr: &syn::Expr, t: &BodyTranslator) -> bool {
    if always_leaves(expr) {
        return true;
    }
    match expr {
        syn::Expr::If(if_expr) => match &if_expr.else_branch {
            Some((_, otherwise)) => {
                block_leaves_in_return_position(&if_expr.then_branch, t)
                    && leaves_in_return_position(otherwise, t)
            }
            None => false,
        },
        syn::Expr::Match(match_expr) => {
            !match_expr.arms.is_empty()
                && match_expr.arms.iter().all(|arm| leaves_in_return_position(&arm.body, t))
        }
        syn::Expr::Block(block) => block_leaves_in_return_position(&block.block, t),
        syn::Expr::Unsafe(block) => block_leaves_in_return_position(&block.block, t),
        syn::Expr::TryBlock(block) => block_leaves_in_return_position(&block.block, t),
        syn::Expr::Group(group) => leaves_in_return_position(&group.expr, t),
        syn::Expr::Paren(paren) => leaves_in_return_position(&paren.expr, t),
        _ => hands_a_value_back(expr, t),
    }
}

/// A block leaves when one of its statements does, or when its TAIL does — the
/// tail is the block's value, and the position wrote a `return` for it.
fn block_leaves_in_return_position(block: &syn::Block, t: &BodyTranslator) -> bool {
    if block_always_leaves(block) {
        return true;
    }
    matches!(block.stmts.last(), Some(syn::Stmt::Expr(tail, None)) if leaves_in_return_position(tail, t))
}

/// Is this expression's Rust value something other than `()`? Asked the way
/// `in_value_position` asks it, which is what decides whether a `return` was
/// written: a type the engine could not resolve is not `()`.
fn hands_a_value_back(expr: &syn::Expr, t: &BodyTranslator) -> bool {
    let mark = t.mark();
    let unit = matches!(t.resolve_expr_type(expr), Ok(crate::ty::Ty::Unit));
    t.rewind(mark);
    !unit
}

/// Does EVERY path out of this expression leave the block it stands in — a
/// `return`, a `break`, a `continue`, or a macro that never comes back?
///
/// The point of the word "every": an `if` with no `else` falls through when its
/// test fails, and a body that ends in one has not finished. Reading the text
/// backwards could not tell the two apart.
pub(crate) fn always_leaves(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Return(_) | syn::Expr::Break(_) | syn::Expr::Continue(_) => true,
        syn::Expr::Macro(mac) => crate::macros::items::never_comes_back(&mac.mac.path),
        syn::Expr::Block(block) => block_always_leaves(&block.block),
        syn::Expr::Unsafe(block) => block_always_leaves(&block.block),
        syn::Expr::TryBlock(block) => block_always_leaves(&block.block),
        syn::Expr::Group(group) => always_leaves(&group.expr),
        syn::Expr::Paren(paren) => always_leaves(&paren.expr),
        // Both halves, or neither: an `if` with no `else` runs on when its test
        // fails, and one whose `else` runs on does too.
        syn::Expr::If(if_expr) => match &if_expr.else_branch {
            Some((_, otherwise)) => {
                block_always_leaves(&if_expr.then_branch) && always_leaves(otherwise)
            }
            None => false,
        },
        // Rust's match is exhaustive, so if every arm leaves, the match does.
        // A match with no arms diverges too, but the port does not write one.
        syn::Expr::Match(match_expr) => {
            !match_expr.arms.is_empty() && match_expr.arms.iter().all(|arm| always_leaves(&arm.body))
        }
        // `loop { .. }` with no `break` of its own never finishes, which is how
        // Rust types it as `!`. A `for` or a `while` may run zero turns.
        syn::Expr::Loop(loop_expr) => !breaks_out(loop_expr),
        _ => false,
    }
}

/// Does every path out of this block leave? A statement that leaves makes the
/// statements below it unreachable, so one is enough.
fn block_always_leaves(block: &syn::Block) -> bool {
    block.stmts.iter().any(|stmt| match stmt {
        syn::Stmt::Expr(expr, _) => always_leaves(expr),
        _ => false,
    })
}

/// Does this `loop` carry a `break` that leaves IT — as opposed to one written
/// inside a loop or a closure nested in it?
fn breaks_out(loop_expr: &syn::ExprLoop) -> bool {
    struct Found(bool);
    impl syn::visit::Visit<'_> for Found {
        fn visit_expr_break(&mut self, _: &syn::ExprBreak) {
            self.0 = true;
        }
        fn visit_expr_loop(&mut self, _: &syn::ExprLoop) {}
        fn visit_expr_while(&mut self, _: &syn::ExprWhile) {}
        fn visit_expr_for_loop(&mut self, _: &syn::ExprForLoop) {}
        fn visit_expr_closure(&mut self, _: &syn::ExprClosure) {}
    }
    let mut found = Found(false);
    syn::visit::visit_block(&mut found, &loop_expr.body);
    found.0
}

#[cfg(test)]
mod tests {
    use super::always_leaves;

    fn expr(src: &str) -> syn::Expr {
        syn::parse_str(src).expect("the test's Rust parses")
    }

    #[test]
    fn an_if_with_no_else_does_not_leave() {
        // K2: `W::Light(n) if *n > 3 => { if *n > 100 { return 1; } }` — the
        // chain wrote no `break` after this arm, so the arm below it ran too.
        assert!(!always_leaves(&expr("{ if n > 100 { return 1; } }")));
        assert!(!always_leaves(&expr("if n > 100 { return 1; }")));
    }

    #[test]
    fn an_if_whose_both_halves_leave_does_leave() {
        assert!(always_leaves(&expr("if n > 100 { return 1; } else { return 2; }")));
        assert!(!always_leaves(&expr("if n > 100 { return 1; } else { n }")));
    }

    #[test]
    fn a_jump_and_a_diverging_macro_leave() {
        assert!(always_leaves(&expr("return 1")));
        assert!(always_leaves(&expr("break")));
        assert!(always_leaves(&expr("continue")));
        assert!(always_leaves(&expr("panic!(\"no\")")));
        assert!(always_leaves(&expr("unreachable!()")));
        assert!(!always_leaves(&expr("println!(\"hi\")")));
    }

    #[test]
    fn a_block_leaves_when_any_statement_of_it_does() {
        assert!(always_leaves(&expr("{ let a = 1; return a; }")));
        assert!(!always_leaves(&expr("{ let a = 1; a }")));
        // What stands after the jump is unreachable, so one is enough.
        assert!(always_leaves(&expr("{ return 1; }")));
    }

    #[test]
    fn a_match_leaves_only_when_every_arm_does() {
        assert!(always_leaves(&expr("match n { 1 => return 1, _ => return 2 }")));
        assert!(!always_leaves(&expr("match n { 1 => return 1, _ => 2 }")));
    }

    #[test]
    fn a_loop_with_no_break_of_its_own_never_finishes() {
        assert!(always_leaves(&expr("loop { work(); }")));
        assert!(!always_leaves(&expr("loop { if done() { break; } }")));
        // A `break` belonging to a nested loop is not this loop's.
        assert!(always_leaves(&expr("loop { while x { break; } }")));
        // A `for` may run no turns at all.
        assert!(!always_leaves(&expr("for x in xs { return 1; }")));
    }
}
