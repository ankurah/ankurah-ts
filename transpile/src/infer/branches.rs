//! What a branching expression is, and the one rule under both forms.
//!
//! Every branch that answers answers with the SAME type. That is what gives a
//! branch written as a bare literal the width another branch says, and it is
//! what makes two branches that cannot both be right say so.
//!
//! The branches are read with their unknowns still standing, so a join that
//! cannot be met knows which unknown it stood on and poisons it: read through
//! the settled table, the disagreement had nothing to take back and was said
//! again at every later use.

use syn::spanned::Spanned;

use super::context::TypeContext;
use crate::diag::Diag;
use crate::ty::Ty;

impl TypeContext<'_> {
    /// What a `match` expression is: the first arm that answers with a type.
    ///
    /// Each arm's body is typed with the arm's OWN names bound. A binding that
    /// shadows an outer one is a different value, and reading the outer one
    /// made the engine report valid Rust as contradicting itself.
    pub(super) fn match_type(
        &self,
        m: &syn::ExprMatch,
        expected: Option<&Ty>,
    ) -> Result<Ty, Diag> {
        let scrutinee = self.resolve_expr(&m.expr).ok();
        let answers: Vec<(proc_macro2::Span, Ty)> = m
            .arms
            .iter()
            .filter_map(|arm| {
                let bound = self.names_a_pattern_binds(&arm.pat, scrutinee.as_ref());
                let found = self.with_question_scope(&bound, || {
                    self.resolve_expr_as_written_expecting(&arm.body, expected).ok()
                });
                found
                    .filter(|t| *t != Ty::Never)
                    .map(|t| (syn::spanned::Spanned::span(&arm.body), t))
            })
            .collect();
        let (_, join) = answers.first().cloned().ok_or_else(|| {
            self.refuse(
                syn::spanned::Spanned::span(m),
                "no arm of this match has a type the engine could read",
            )
        })?;
        // Every arm that answers answers with the SAME type, which is what
        // gives an arm written as a bare literal the width another arm says.
        for (span, found) in answers.iter().skip(1) {
            self.constrain_here(*span, &join, found);
        }
        Ok(join)
    }

    /// What an `if` expression is: the branch that answers, with the other
    /// branch held to it. An `if` with no `else` is the unit type.
    ///
    /// An `if let`'s then-branch is read with the pattern's OWN names bound, as
    /// a match arm's body is. A name the pattern shadows is a different value,
    /// and reading the outer one reported valid Rust as contradicting itself.
    pub(super) fn if_type(
        &self,
        if_expr: &syn::ExprIf,
        expected: Option<&Ty>,
    ) -> Result<Ty, Diag> {
        let bound = match &*if_expr.cond {
            syn::Expr::Let(let_expr) => {
                let scrutinee = self.resolve_expr(&let_expr.expr).ok();
                self.names_a_pattern_binds(&let_expr.pat, scrutinee.as_ref())
            }
            _ => Vec::new(),
        };
        let then = self.with_question_scope(&bound, || {
            self.resolve_block_as_written_expecting(&if_expr.then_branch, expected)
        });
        let other = match &if_expr.else_branch {
            Some((_, other)) => self.resolve_expr_as_written_expecting(other, expected),
            None => Ok(Ty::Unit),
        };
        if let (Ok(then), Ok(other)) = (&then, &other) {
            if *then != Ty::Never && *other != Ty::Never {
                self.constrain_here(if_expr.then_branch.span(), then, other);
            }
        }

        match &then {
            Ok(ty) if *ty != Ty::Never => then,
            _ => other,
        }
    }
}
