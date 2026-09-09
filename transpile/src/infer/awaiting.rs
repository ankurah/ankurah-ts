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
        if let Some(output) = output {
            return Ok(output);
        }
        // A call to a declared `async fn` returns what it writes (spec 4.10),
        // so awaiting one is the identity.
        if self.awaits_a_declared_async_call(&await_expr.base) {
            return Ok(base);
        }
        Err(self.refuse(
            at,
            format!(
                "`.await` on `{}`, whose `Future` the engine cannot read",
                self.registry.describe(&base)
            ),
        ))
    }

    /// Is this the call of a declared `async fn`?
    ///
    /// Asked quietly: what the callee cannot be read as is the call's own
    /// failure to report, not this `await`'s.
    pub(super) fn awaits_a_declared_async_call(&self, base: &syn::Expr) -> bool {
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
    }}
