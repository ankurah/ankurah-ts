//! What `.await` answers with.
//!
//! This port models a call to an `async fn` as returning what it writes rather
//! than a future (spec 4.10), so awaiting one is the identity. Anything else
//! needs a `Future` the impl table can read, or the site is one the engine
//! cannot type: taking the base typed it as the wrapper, not as what it holds.

use super::context::TypeContext;
use crate::diag::Diag;
use crate::ty::Ty;

impl TypeContext<'_> {
    /// The type an `.await` answers with, at the span of the whole expression.
    pub(super) fn await_type(
        &self,
        await_expr: &syn::ExprAwait,
        at: proc_macro2::Span,
    ) -> Result<Ty, Diag> {
        let base = self.resolve_expr(&await_expr.base)?;
        let output = self
            .project_through(&base, "std::future::Future", "Output")
            // A projection the impl table did not read through comes back as a
            // projection, which says less than the base does.
            .filter(|ty| !ty.mentions_projection() && !ty.mentions_infer());
        // A call to a declared `async fn` returns what it writes (spec 4.10),
        // so awaiting one is the identity. Read FIRST: a declared result that
        // is ITSELF a future would otherwise be projected a second time, and
        // the written `.await` would answer what two awaits answer.
        if self.awaits_a_declared_async_call(&await_expr.base) {
            let Some(inner) = output else { return Ok(base) };
            return Err(self.refuse(
                at,
                assimilated_future(&self.registry.describe(&base), &self.registry.describe(&inner)),
            ));
        }
        if let Some(output) = output {
            return Ok(output);
        }
        Err(self.refuse(
            at,
            format!(
                "`.await` on `{}`, whose `Future` the engine cannot read",
                self.registry.describe(&base)
            ),
        ))
    }

    /// The refusal a declared `async fn` owes when what it writes is ITSELF a
    /// future, read off its written return type.
    ///
    /// The await where such a call is read already says this. The FUNCTION owes
    /// it too: emitted, its body answers the inner value, which is the value
    /// JavaScript's `async` assimilates.
    pub fn writes_an_assimilated_future(&self, written: Option<&syn::Type>) -> Option<String> {
        let mark = self.sink.mark();
        let resolved = written.and_then(|ty| self.resolve_written_type(ty).ok());
        self.sink.rewind(mark);
        let resolved = resolved?;
        let inner = self
            .project_through(&resolved, "std::future::Future", "Output")
            .filter(|ty| !ty.mentions_projection() && !ty.mentions_infer())?;
        Some(assimilated_future(
            &self.registry.describe(&resolved),
            &self.registry.describe(&inner),
        ))
    }

    /// Is this the call of a declared `async fn`?
    ///
    /// Asked quietly: what the callee cannot be read as is the call's own
    /// failure to report, not this `await`'s.
    pub fn awaits_a_declared_async_call(&self, base: &syn::Expr) -> bool {
        let mark = self.sink.mark();
        let found = match super::calls::unparenthesise(base) {
            syn::Expr::MethodCall(call) => self
                .resolve_method_call_with(&call.receiver, &call.method.to_string(), call.turbofish.as_ref())
                .ok()
                .and_then(|found| self.registry.method_sig(&found))
                .is_some_and(|sig| sig.is_async),
            syn::Expr::Call(call) => self
                .call_sig(call, None)
                .is_some_and(|(sig, _)| sig.is_async),
            _ => false,
        };
        self.sink.rewind(mark);
        found
    }
}

/// What an `.await` on a declared `async fn` whose own result is a future says.
///
/// Named so the sentence is written in one place and can be read by a test: as
/// an inline `format!` it had lost two line continuations and carried two runs
/// of twenty-two spaces into the diagnostics.
pub fn assimilated_future(outer: &str, inner: &str) -> String {
    format!(
        "`{}` is what this declared `async fn` answers with and is itself a future of `{}`; \
         JavaScript's own `async` awaits such a value before the caller does, so the port has \
         nothing to hold the outer future in",
        outer, inner
    )
}

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

    /// A diagnostic is a sentence a person reads, so a lost line continuation
    /// is a defect: the run of spaces it leaves behind is printed verbatim.
    #[test]
    fn the_refusal_for_an_assimilated_future_is_not_mangled() {
        let message = super::assimilated_future("impl Future<Output = u64>", "u64");
        assert!(!message.contains("  "), "the message is mangled: {message}");
    }

    /// Emitted, such a body answers the INNER value, which is the one
    /// JavaScript's `async` assimilates before the caller's own await runs.
    #[test]
    fn an_async_fn_that_writes_a_future_is_a_hole() {
        let mut c = Fixture::build(&[(
            "lib.rs",
            "pub async fn writes_a_future(n: u64) -> impl std::future::Future<Output = u64> \
             { async move { n + 1 } }",
        )]);
        assert_eq!(
            c.translated_method("lib.rs", "writes_a_future").trim(),
            format!("return {};", crate::body::hole_text(&super::assimilated_future(
                "impl Future<Output = u64>",
                "u64"
            )))
        );
    }
}
