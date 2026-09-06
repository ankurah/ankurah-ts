//! What the value being taken apart IS, and the scope its pattern opens.
//!
//! For: a `match`, an `if let`, a `while let` and a `for` loop all point a
//! pattern at a value, and everything about who owns what the pattern binds
//! follows from that value's type — with the written `&` still on it. Rust's
//! default binding mode (RFC 2005) makes every name under a pattern matched
//! against a reference a borrow, and the port erases references everywhere
//! else, so this is the one place that has to keep them.

use super::{BodyTranslator, PatternScope};

impl<'a> BodyTranslator<'a> {
    /// The type of an expression a PATTERN is matched against, with a written
    /// `&` kept on it.
    ///
    /// For: everywhere else the port erases references, because a TypeScript
    /// object reference is what a Rust reference becomes, so `resolve_expr_type`
    /// answers what a `&e` points at. Matching is where the difference decides
    /// who owns what the pattern binds: Rust's default binding mode (RFC 2005)
    /// says a pattern matched against a reference binds by reference, and the
    /// binding then owes no release. With the `&` erased, `for v in &map`
    /// released the borrowed key and value on every turn and the map released
    /// them again, and `if let Some(order_by) = &self.order_by` released a
    /// vector the field still holds — both double drops the strict registry
    /// aborts on.
    pub fn borrowed_scrutinee_type(&self, expr: &syn::Expr) -> Option<crate::ty::Ty> {
        match expr {
            syn::Expr::Reference(r) => Some(crate::ty::Ty::Ref {
                mutable: r.mutability.is_some(),
                inner: Box::new(self.borrowed_scrutinee_type(&r.expr)?),
            }),
            syn::Expr::Paren(p) => self.borrowed_scrutinee_type(&p.expr),
            syn::Expr::Group(g) => self.borrowed_scrutinee_type(&g.expr),
            // A tuple written to be matched is a tuple of scrutinees, and each
            // element carries its own `&`: `if let (Ex::Path(p), Ex::Lit(q)) =
            // (&**left, &**right)` matches two BORROWED enums, so `p` and `q`
            // are borrows and the branch releases neither. Asking
            // `resolve_expr_type` for the whole tuple erased both references and
            // typed the two names as owned payloads, and the branch dropped
            // values their owners still hold.
            syn::Expr::Tuple(tuple) => {
                let elems: Option<Vec<crate::ty::Ty>> =
                    tuple.elems.iter().map(|e| self.borrowed_scrutinee_type(e)).collect();
                match elems {
                    Some(elems) if !elems.is_empty() => Some(crate::ty::Ty::Tuple(elems)),
                    _ => self.scrutinee_type(expr),
                }
            }
            _ => self.scrutinee_type(expr),
        }
    }

    /// The type of the expression a `for` loop iterates.
    ///
    /// `IntoIterator for Vec<T>` hands out a `T` the loop has to release, and
    /// `IntoIterator for &Vec<T>` a `&T` that stays the sequence's, so the
    /// written `&` decides the form of the whole loop.
    pub fn iterated_type(&self, iterated: &syn::Expr) -> Option<crate::ty::Ty> {
        self.borrowed_scrutinee_type(iterated)
    }

    /// What one turn of a `for` loop over this expression hands out.
    pub fn iteration_item(&self, iterated: &syn::Expr) -> Option<crate::ty::Ty> {
        let tc = self.types.as_ref()?;
        let ty = self.iterated_type(iterated)?;
        // A sequence the engine cannot name the element of leaves the loop
        // variable untyped; the uses of that variable are what report it, so
        // that one gap is counted once per site rather than twice.
        tc.borrow().iteration_item(&ty)
    }

    /// The type of the value a `match` or an `if let` takes apart. `None` means
    /// the engine could not read it, and the fallback has been recorded.
    pub fn scrutinee_type(&self, expr: &syn::Expr) -> Option<crate::ty::Ty> {
        let resolved = self.resolve_expr_type(expr);
        self.or_fallback(resolved, "the pattern's names are bound without types")
    }

    /// Write `body` with the pattern lowering told what the value being taken
    /// apart IS, for a site that writes the test after its binding scope has
    /// closed.
    ///
    /// `enter_pattern` sets the same flag, and most sites write the test inside
    /// the scope it opens; the `if let` lowering writes its branch first and its
    /// test afterwards, so it says so here instead.
    pub(crate) fn matching<R>(
        &self,
        scrutinee: Option<&crate::ty::Ty>,
        body: impl FnOnce() -> R,
    ) -> R {
        let held = self.subject_ty.replace(scrutinee.cloned());
        let written = body();
        *self.subject_ty.borrow_mut() = held;
        written
    }

    /// The same, for ONE element of a tuple subject: `(&*left, &*right)` is a
    /// tuple whose elements are references and which is not one itself, so the
    /// element's own type is what says whether its pattern binds by reference
    /// (K16). A subject the engine could not read as a tuple leaves the answer
    /// where it was.
    pub(crate) fn matching_element<R>(&self, at: usize, body: impl FnOnce() -> R) -> R {
        let element = match &*self.subject_ty.borrow() {
            Some(crate::ty::Ty::Tuple(elements)) => elements.get(at).cloned(),
            // A `&(A, B)` binds each element by reference too.
            Some(crate::ty::Ty::Ref { inner, .. }) => match &**inner {
                crate::ty::Ty::Tuple(elements) => elements.get(at).map(|ty| crate::ty::Ty::Ref {
                    mutable: false,
                    inner: Box::new(ty.clone()),
                }),
                _ => None,
            },
            _ => None,
        };
        let Some(element) = element else { return body() };
        let held = self.subject_ty.replace(Some(element));
        let written = body();
        *self.subject_ty.borrow_mut() = held;
        written
    }

    /// Is the value the pattern being written is matched against borrowed?
    ///
    /// `enter_pattern` sets it from the scrutinee's type and the scope it opens
    /// restores it, so it answers for the pattern currently being written.
    pub(crate) fn matches_a_reference(&self) -> bool {
        matches!(*self.subject_ty.borrow(), Some(crate::ty::Ty::Ref { .. }))
    }

    /// Open a scope holding the names a pattern introduces, typed from the value
    /// being taken apart. The scope closes when the returned guard drops.
    ///
    /// A name the engine could not type is still bound, so that it shadows and
    /// so that a use of it says "bound but untyped" rather than "does not name a
    /// value". The gap is reported where the name is used, which is where the
    /// translator has to fall back; reporting it here as well would count one
    /// gap twice.
    pub fn enter_pattern<'t>(
        &'t self,
        pat: &syn::Pat,
        scrutinee: Option<&crate::ty::Ty>,
    ) -> PatternScope<'t, 'a> {
        self.push_block();
        let subject_before = self.subject_ty.replace(scrutinee.cloned());
        self.bind_pattern_here(pat, scrutinee);
        PatternScope { translator: self, subject_before }
    }

    /// Bind a pattern's names in the scope that is already open. Used where the
    /// binding outlives the statement, as a `let` does.
    pub fn bind_pattern_here(&self, pat: &syn::Pat, scrutinee: Option<&crate::ty::Ty>) {
        let Some(tc) = &self.types else { return };
        tc.borrow_mut().bind_pattern(pat, scrutinee);
    }

    /// Does this pattern take a payload out of the scrutinee that the arm then
    /// owns, rather than binding the whole value?
    pub(crate) fn pattern_takes_a_payload(&self, expr: &syn::Expr, pat: &syn::Pat) -> bool {
        let Some(tc) = &self.types else { return false };
        let Ok(subject) = self.quietly(|| self.resolve_expr_type(expr)) else {
            return false;
        };
        let tc = tc.borrow();
        let takes = crate::ownership::scrutinee::takes(&tc.probe(), &subject, &[pat], |path| {
            let mark = tc.sink.mark();
            let payload = tc.payload_of(path, Some(&subject));
            tc.sink.rewind(mark);
            // The names travel with the types: a struct pattern names its
            // members, and the payload's order is the declaration's (I2).
            payload.unwrap_or_default()
        });
        takes == crate::ownership::scrutinee::Takes::Payload
    }

}
