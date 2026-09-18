//! What a block written as an EXPRESSION is.
//!
//! For: a block read from outside has bound none of its own names, so
//! `let subscribers = { let listeners = ..; listeners.values()... }` could not
//! say what its tail was and left the binding untyped. Its `let`s are bound in
//! order before the tail is read, by both walks, so the tail sees what the
//! source wrote above it.

use super::context::TypeContext;
use crate::diag::Diag;
use crate::ty::Ty;

impl TypeContext<'_> {
    /// The block's tail, settled through the table.
    pub(super) fn resolve_block_expecting(
        &self,
        block: &syn::Block,
        expected: Option<&Ty>,
    ) -> Result<Ty, Diag> {
        self.with_the_blocks_own_lets(block, || match block.stmts.last() {
            Some(syn::Stmt::Expr(tail, None)) => self.resolve_expr_expecting(tail, expected),
            _ => Ok(Ty::Unit),
        })
    }

    /// The same, with the body's unknowns still standing while the solve runs.
    pub(super) fn resolve_block_as_written_expecting(
        &self,
        block: &syn::Block,
        expected: Option<&Ty>,
    ) -> Result<Ty, Diag> {
        self.with_the_blocks_own_lets(block, || match block.stmts.last() {
            Some(syn::Stmt::Expr(tail, None)) => {
                self.resolve_expr_as_written_expecting(tail, expected)
            }
            _ => Ok(Ty::Unit),
        })
    }

    /// Ask something about a block with the block's own `let`s bound, each one
    /// seeing the ones written above it.
    pub(super) fn with_the_blocks_own_lets<T>(
        &self,
        block: &syn::Block,
        ask: impl FnOnce() -> T,
    ) -> T {
        let mut frames = 0;
        for stmt in &block.stmts {
            let syn::Stmt::Local(local) = stmt else { continue };
            let ty = self.resolve_local_type(local).ok();
            let bound = self.names_a_pattern_binds(&local.pat, ty.as_ref());
            self.question_scope.borrow_mut().push(bound);
            frames += 1;
        }
        let answer = ask();
        for _ in 0..frames {
            self.question_scope.borrow_mut().pop();
        }
        answer
    }
}
