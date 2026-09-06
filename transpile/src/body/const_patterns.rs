//! A name in a pattern that resolves to a `const`, and what testing against it
//! costs.
//!
//! For: Rust resolves a pattern's identifier in the VALUE namespace first, so
//! `ORIGIN => ..` is a comparison against the const's value and not a binding.
//! Read as a binding it bound `oRIGIN`, matched everything, and made every arm
//! below it unreachable — the only diagnostic named the wrong arm. How the
//! comparison is written depends on the const's declared type, and for a type
//! the port compares by identity it cannot be written at all.

use super::BodyTranslator;

impl<'a> BodyTranslator<'a> {
    /// Does this path name a `const` or a `static`, rather than a binding?
    ///
    /// Rust resolves a pattern's identifier in the VALUE namespace first: a
    /// name that lands on a const is a comparison against its value, and only a
    /// name that lands on nothing binds. The registry's value namespace holds
    /// consts, statics and free functions; a function has a signature and a
    /// const does not, which is what tells them apart.
    ///
    /// The answer carries the const's declared type, because that is what
    /// decides how the comparison is written.
    pub(crate) fn names_a_const(&self, segments: &[String]) -> Option<Option<crate::ty::Ty>> {
        let tc = self.types.as_ref()?;
        let tc = tc.borrow();
        let mark = tc.sink.mark();
        let found = tc
            .registry
            .lookup(tc.module, crate::registry::Ns::Value, segments);
        tc.sink.rewind(mark);
        match found {
            Ok(Some(crate::registry::Def::Value(id))) => {
                let value = tc.registry.value(id)?;
                // A free function is in the value namespace too, and naming one
                // in a pattern is not a comparison.
                if value.sig.is_some() {
                    return None;
                }
                Some(value.ty.clone())
            }
            _ => None,
        }
    }

    /// Is every use of this name a fresh value, so the emitted name is a
    /// function this use calls? See `ValueDef::fresh_at_each_use`.
    pub(crate) fn names_a_fresh_const(&self, segments: &[String]) -> bool {
        let Some(tc) = self.types.as_ref() else { return false };
        let tc = tc.borrow();
        let mark = tc.sink.mark();
        let found = tc
            .registry
            .lookup(tc.module, crate::registry::Ns::Value, segments);
        tc.sink.rewind(mark);
        match found {
            Ok(Some(crate::registry::Def::Value(id))) => tc
                .registry
                .value(id)
                .is_some_and(|value| value.fresh_at_each_use),
            _ => false,
        }
    }

    /// The test a const pattern writes: the subject against the const's value.
    ///
    /// Where the const's type is one the port compares by identity there is no
    /// test to write, and the refusal goes in the BRANCH — D2's rule, because a
    /// hole in a condition is a `never` the branch's bindings sit under.
    pub(crate) fn const_pattern_test(
        &self,
        subject: &str,
        segments: &[String],
        pat: &syn::Pat,
    ) -> (String, String) {
        let name = crate::name_map::escape_reserved(segments.last().expect("a path has a segment"));
        let ty = self.names_a_const(segments).flatten();
        let compares_by_identity = matches!(
            ty.as_ref().map(|t| t.peel_refs()),
            Some(crate::ty::Ty::Prim(_)) | Some(crate::ty::Ty::Str) | None
        );
        if compares_by_identity {
            return (format!("{} === {}", subject, name), String::new());
        }
        // R12: the arm says so and stops rather than answering what Rust would
        // not.
        let hole = self.hole(
            syn::spanned::Spanned::span(pat),
            format!(
                "`{}` is a const of a type the port compares by identity, and Rust compares a \
                 const pattern by value",
                segments.join("::")
            ),
        );
        ("true".to_string(), format!("{};\n", hole))
    }
}
