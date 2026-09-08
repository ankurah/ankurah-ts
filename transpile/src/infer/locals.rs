//! What a `let` binds, and what says so.
//!
//! A binding's type is its annotation where the source wrote one and its
//! initialiser's otherwise. Which of the two answers matters to everything
//! below the statement: it is what a later use of the name resolves through,
//! and what says whether the block owes the value a release.

use syn::spanned::Spanned;

use super::context::TypeContext;
use super::expected;
use crate::diag::Diag;
use crate::ty::Ty;

impl TypeContext<'_> {
    /// The type of a `let` binding: its annotation if it has one, otherwise
    /// the type of what initialises it.
    pub fn resolve_local_type(&self, local: &syn::Local) -> Result<Ty, Diag> {
        let annotated = self.local_annotation(local);
        match (annotated, &local.init) {
            // A `let x: Vec<_> = ..` says most of the type and leaves a hole
            // for the initialiser to close.
            (Some(written), Some(init)) if expected::has_infer(&written) => {
                let filled = self.resolve_expr_expecting(&init.expr, Some(&written))?;
                Ok(expected::fill_infer(&written, &filled))
            }
            (Some(written), _) => Ok(written),
            (None, Some(init)) => self.resolve_expr(&init.expr),
            (None, None) => Err(self.refuse(
                local.span(),
                "binding has neither a type nor an initialiser",
            )),
        }
    }

    /// The type a `let` writes for itself, which is what its initialiser is
    /// expected to produce.
    pub fn local_annotation(&self, local: &syn::Local) -> Option<Ty> {
        match &local.pat {
            syn::Pat::Type(pat_type) => self.resolve_written_type(&pat_type.ty).ok(),
            _ => None,
        }
    }
}
