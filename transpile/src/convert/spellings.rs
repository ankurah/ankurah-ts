//! What a type IS in the port, asked where a conversion has to decide.
//!
//! For: a `?`, a literal's width and a wrapper's payload each need one answer
//! about the runtime shape a Rust type takes, and holding those answers
//! together keeps the conversions from each inventing their own.

use super::BodyTranslator;

impl BodyTranslator<'_> {
    /// What a `?` operand has to be, given what the `?` itself has to produce.
    pub(crate) fn try_operand_expectation(
        &self,
        expected: Option<&crate::ty::Ty>,
    ) -> Option<crate::ty::Ty> {
        self.types
            .as_ref()?
            .borrow()
            .try_operand_expectation(expected)
    }

    /// Is this integer literal one the port writes as a `bigint`?
    ///
    /// The written suffix decides it where there is one; otherwise it is what
    /// the position wants, which is how `n + 1` beside a `u64` writes `1n`, and
    /// where the position says nothing it is what the SOLVE settled the
    /// literal to — the same width the arithmetic helpers are told.
    pub(crate) fn is_bigint_literal(&self, lit: &syn::Lit, expected: Option<&crate::ty::Ty>) -> bool {
        let syn::Lit::Int(int) = lit else { return false };
        match int.suffix() {
            "u64" | "i64" | "u128" | "i128" => return true,
            "" => {}
            _ => return false,
        }
        let settled = expected.cloned().or_else(|| {
            self.types
                .as_ref()?
                .borrow()
                .settled_literal(syn::spanned::Spanned::span(lit))
        });
        matches!(
            settled.as_ref().map(crate::ty::Ty::peel_refs),
            Some(crate::ty::Ty::Prim(
                crate::ty::Prim::U64
                    | crate::ty::Prim::I64
                    | crate::ty::Prim::U128
                    | crate::ty::Prim::I128
            ))
        )
    }

    /// Is this the `Result<T, E>` the port writes as the runtime's `Result`?
    pub(crate) fn is_result(&self, ty: &crate::ty::Ty) -> bool {
        match &self.types {
            Some(tc) => tc.borrow().is_result(ty),
            None => false,
        }
    }

    /// Is this the `Option<T>` the port writes as `T | null`?
    pub(crate) fn is_nullable(&self, ty: &crate::ty::Ty) -> bool {
        let Some(tc) = &self.types else { return false };
        let Some(id) = ty.peel_refs().id() else {
            return false;
        };
        matches!(
            tc.borrow().registry.shapes().form(id),
            Some(crate::name_map::system_shapes::Form::Nullable)
        )
    }}
