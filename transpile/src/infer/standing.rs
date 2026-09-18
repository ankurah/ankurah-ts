//! Where the walk is standing, and what that position is read for.
//!
//! A question asked deep inside an expression has no position of its own, and
//! two reports need one: a constraint the walk that writes a body meets that
//! the solve never saw, and an answer the solve read through a bound that later
//! turns out false. Both are found far below the expression that has to say
//! them, so the walk keeps the expressions it is inside.

use syn::spanned::Spanned;

use super::context::TypeContext;

impl TypeContext<'_> {
    /// Answer a question about `expr` with the walk standing at it, so what is
    /// found deep inside is reported where the source wrote it.
    pub(super) fn inside<T>(&self, expr: &syn::Expr, answer: impl FnOnce() -> T) -> T {
        self.standing_at.borrow_mut().push(expr.span());
        let out = answer();
        self.standing_at.borrow_mut().pop();
        out
    }

    /// The innermost expression the walk is inside.
    pub(super) fn standing_at(&self) -> Option<proc_macro2::Span> {
        self.standing_at.borrow().last().copied()
    }

    /// Answer a question asked about an expression rather than from inside it,
    /// so what the answer turns out to stand on is still reported there.
    pub(super) fn standing<T>(&self, span: proc_macro2::Span, answer: impl FnOnce() -> T) -> T {
        self.standing_at.borrow_mut().push(span);
        let out = answer();
        self.standing_at.borrow_mut().pop();
        out
    }

    /// Note that the walk that writes a body met a constraint the solve never
    /// saw, at the innermost expression it is inside.
    pub(super) fn note_unsolved_site(&self) {
        if let Some(at) = self.standing_at() {
            self.unsolved_sites.borrow_mut().push(at);
        }
    }

    /// Where such constraints have been met so far.
    pub fn unsolved_sites(&self) -> Vec<proc_macro2::Span> {
        self.unsolved_sites.borrow().clone()
    }
}
