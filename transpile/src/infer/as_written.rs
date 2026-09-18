//! Reading an expression with the body's unknowns still standing.
//!
//! For: a constraint raised over SOLVED types no longer says which unknown it
//! stood on, so a contradiction between two of them poisons nothing and every
//! later constraint on the same unknown reports the same disagreement again.
//! These readers answer the solve with the unknowns in place. The walk that
//! WRITES a body takes the settled answer from them instead, because an unknown
//! offered to that walk is one it could bind.

use super::context::TypeContext;
use crate::diag::Diag;
use crate::ty::Ty;

impl TypeContext<'_> {
    /// What an expression is, with its unknowns in place while the SOLVE runs.
    pub fn resolve_expr_as_written(&self, expr: &syn::Expr) -> Result<Ty, Diag> {
        self.resolve_expr_as_written_expecting(expr, None)
    }

    /// The same, where the position says what the expression has to be.
    pub(super) fn resolve_expr_as_written_expecting(
        &self,
        expr: &syn::Expr,
        expected: Option<&Ty>,
    ) -> Result<Ty, Diag> {
        let found = self.inside(expr, || self.expr_type(expr, expected))?;
        match self.vars.borrow().solving() {
            true => Ok(found),
            false => Ok(self.solved(&found)),
        }
    }

}
