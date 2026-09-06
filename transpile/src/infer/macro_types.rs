//! What a MACRO's expression is worth, as a type.
//!
//! The port expands no macro (R6), so a macro in a value position has no
//! body to type: what it is worth is written down here, once, for the
//! macros the corpus uses. A macro not in this table is refused rather than
//! guessed at, because guessing would give the position it stands in a type
//! nothing checked.

use super::expected;
use super::TypeContext;
use crate::diag::Diag;
use crate::ty::{Prim, Ty};

impl TypeContext<'_> {
    /// What a macro invocation produces (spec 4.10). The transpiler never
    /// expands one, so each supported macro's type is stated here and every
    /// other macro is refused at the invocation.
    pub(super) fn macro_type(&self, mac: &syn::Macro, expected: Option<&Ty>) -> Result<Ty, Diag> {
        let span = syn::spanned::Spanned::span(mac);
        let name = mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        match name.as_str() {
            // `vec![..]` is a `Vec` of whatever its elements are, and its
            // elements are whatever the position wants: `vec![1, 2]` behind a
            // `Vec<u8>` holds bytes, not the `i32`s a bare literal defaults to.
            "vec" => {
                let elem_want = expected.and_then(|ty| expected::element_of(self.registry, ty));
                let id = self.registry.system_type("std::vec::Vec").ok_or_else(|| {
                    self.refuse(span, "`vec!` yields a Vec, which is not declared")
                })?;
                let elems = crate::macros::vec_macro_elements(mac);
                let elem = match (elems.first(), elem_want) {
                    (Some(first), want) => self.resolve_expr_expecting(first, want.as_ref())?,
                    (None, Some(want)) => want,
                    (None, None) => {
                        return Err(self.refuse(
                            span,
                            "an empty `vec![]` has no element type, and the position it stands \
                             in does not say one",
                        ))
                    }
                };
                Ok(Ty::Named {
                    id,
                    args: vec![elem],
                })
            }
            "format" => self
                .registry
                .system_type("std::string::String")
                .map(|id| Ty::Named { id, args: Vec::new() })
                .ok_or_else(|| self.refuse(span, "`format!` yields a String, which is not declared")),
            "panic" | "todo" | "unimplemented" | "unreachable" => Ok(Ty::Never),
            "assert" | "assert_eq" | "assert_ne" | "debug_assert" | "debug_assert_eq"
            | "debug_assert_ne" => Ok(Ty::Unit),
            "matches" => Ok(Ty::Prim(Prim::Bool)),
            // `stringify!(..)` is a `&'static str` — the tokens as written.
            // Typed by nothing, `stringify!($ty).to_owned()` came out
            // `'$ ty'.clone()`, and a JavaScript string has no `clone`: the
            // erasure that makes `"x".to_owned()` the identity is asked of the
            // RECEIVER's type. Live at `core/property/mod.rs`, which writes
            // that line outside the macro that defines `$ty` — so Rust's own
            // answer there is the literal `"$ty"`.
            "stringify" => Ok(Ty::Ref {
                mutable: false,
                inner: Box::new(Ty::Str),
            }),
            "trace" | "debug" | "info" | "warn" | "error" => Ok(Ty::Unit),
            other => Err(self.refuse(
                span,
                format!("`{}!` has no declared type", other),
            )),
        }
    }
}
