//! Reading through a `&*` place, and the gap a cell argument leaves.
//!
//! Split out of `body.rs`, which was over the 600-line rule and grew again when
//! a call's arguments were put back on their own lines (R10: the ratchet is met
//! by splitting, never by joining lines). What is here is the one shape whose
//! answer depends on whether the place is read ONCE or twice: `&*guard` behind a
//! lock reads the guard's value, and reading it twice takes the lock twice.

use super::{BodyTranslator, ASSUMED_ACCESSOR};

impl BodyTranslator<'_> {
    /// A `&mut <place>` handed to a parameter the port holds in a CELL, where
    /// the place is not a local.
    ///
    /// C1 turns `&mut u32` into a `BorrowMut<number>` so the callee's write
    /// reaches the caller, and only a local can be held in one: `&mut c.n`
    /// hands the callee a copy of the number, and the write goes nowhere.
    /// `ownership.md` said this was reported; it was not, and the emitted call
    /// passed a bare `number` to a `BorrowMut<number>` parameter.
    ///
    /// R12: the site says what it could not translate and stops there, rather
    /// than running an update nobody sees.
    pub(super) fn cell_argument_gap(&self, arg: &syn::Expr, want: Option<&crate::ty::Ty>) -> Option<String> {
        let crate::ty::Ty::Ref { mutable: true, inner } = want? else {
            return None;
        };
        let spelled = match &self.types {
            Some(tc) => crate::name_map::map_ty(tc.borrow().registry, inner),
            None => return None,
        };
        if !crate::is_value_spelling(&spelled) {
            return None;
        }
        let syn::Expr::Reference(reference) = arg else { return None };
        if reference.mutability.is_none() {
            return None;
        }
        // A single name is the case C1 covers: the local is held in a cell and
        // the cell is what goes over.
        if matches!(&*reference.expr, syn::Expr::Path(path) if path.path.segments.len() == 1) {
            return None;
        }
        Some(self.hole(
            syn::spanned::Spanned::span(arg),
            format!(
                "`&mut {}` borrows a place that is not a local, and a `&mut` to a value \
                 JavaScript copies is passed as a cell — which only a local can be held in, so \
                 the callee's write would reach nobody",
                spelled
            ),
        ))
    }

    /// The same place, read ONCE.
    ///
    /// `*counts.entry(k).or_insert(0) += 1` is one place in Rust and two
    /// mentions here — `p = f(p, 1)` — so a place with a side effect performed
    /// it twice: the entry was created twice and the key cloned twice, and the
    /// second clone leaked. The receiver is named first where it is not already
    /// a place, and the accessor hangs off the name.
    pub(crate) fn deref_place_read_once(&self, unary: &syn::ExprUnary) -> String {
        if crate::body::is_place(&unary.expr) || self.names_a_cell(&unary.expr) {
            return self.deref_place(unary);
        }
        // `hoist_produced` first, because a temporary with drop glue — a mutex
        // guard — owes a release and a bare `const` would not give it one. Only
        // where it declines does the place get a plain name of its own.
        let written = self.through_place(&unary.expr, || self.expr(&unary.expr));
        let held = self.hoist_produced(&unary.expr, written.clone());
        let inner = if held == written { self.hoist_name(written) } else { held };
        let Some(tc) = &self.types else {
            self.fallback(syn::spanned::Spanned::span(&*unary.expr), ASSUMED_ACCESSOR);
            return format!("{}.value", inner);
        };
        let accessor = tc.borrow().deref_accessor_of(&unary.expr);
        match self.or_fallback(accessor, ASSUMED_ACCESSOR) {
            Some(accessor) => format!("{}.{}", inner, accessor),
            None => format!("{}.value", inner),
        }
    }

    /// `*place`, written as the place an assignment stores into.
    ///
    /// A `*` in a value position may reach through nothing at all — `*x` on a
    /// `&T` is the `T`, and emission erases the reference — so a deref the
    /// engine could not resolve is written as the value itself. An assignment
    /// target cannot be: `*guard = v` and `*guard += 1` store *through* the
    /// wrapper whatever the engine could say about it, and dropping the
    /// accessor there emitted `counter.lock() += 1`, which names no place at
    /// all. So the target keeps `.value` as its default, and says that it
    /// assumed it.
    pub(crate) fn deref_place(&self, unary: &syn::ExprUnary) -> String {
        // C1: a name the body holds in a cell is ALREADY read through it —
        // `path_expr` writes `found.value` — so `*found` is that place and not
        // a second `.value` on top of it.
        if self.names_a_cell(&unary.expr) {
            return self.expr(&unary.expr);
        }
        let inner = self.through_place(&unary.expr, || self.expr(&unary.expr));
        let inner = self.hoist_produced(&unary.expr, inner);
        let Some(tc) = &self.types else {
            self.fallback(syn::spanned::Spanned::span(&*unary.expr), ASSUMED_ACCESSOR);
            return format!("{}.value", inner);
        };
        let accessor = tc.borrow().deref_accessor_of(&unary.expr);
        match self.or_fallback(accessor, ASSUMED_ACCESSOR) {
            Some(accessor) => format!("{}.{}", inner, accessor),
            None => format!("{}.value", inner),
        }
    }
}
