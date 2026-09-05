//! What a call, a match or an operator takes away from the block that held it.
//!
//! For: the move scan reads the SYNTAX — it walks a block looking for the
//! places a value is handed on — and the syntax of `r.unwrap()`, `match e`,
//! `a + b`, `-a` and `f()` says nothing about whether the callee consumes what
//! it was given. The type engine and the impl table say. This is where the two
//! meet: one answer per shape, each asked of the engine and none of them
//! guessed from a name.

use crate::ownership;

use super::BodyTranslator;

/// Does this method call take its receiver by value?
///
/// The move analysis asks; the impl table answers. `Result::unwrap`,
/// `Option::take` and the `into_*` family all take `self`, and a receiver they
/// took is not the block's to release any more.
impl ownership::moves::Consumes for BodyTranslator<'_> {
    fn consumes_receiver(&self, call: &syn::ExprMethodCall) -> bool {
        let Some(tc) = &self.types else { return false };
        let tc = tc.borrow();
        // Asking is not translating. The resolution files the questions it
        // deferred, and this asks the same call several times — once per
        // statement scan, once per flag scan — so the record is wound back to
        // where it stood. The translation of the call reports them once.
        let mark = tc.sink.mark();
        let found = tc.resolve_method_call_with(
            &call.receiver,
            &call.method.to_string(),
            call.turbofish.as_ref(),
        );
        tc.sink.rewind(mark);
        let Ok(found) = found else {
            // `.await` on a named future and `Result::unwrap` on a receiver the
            // engine could not type are the two that matter, and both are worth
            // reading off the name rather than losing: taking a receiver that
            // was not taken leaks, and leaving one that was taken double-drops.
            return matches!(
                call.method.to_string().as_str(),
                "unwrap" | "expect" | "unwrap_err" | "expect_err" | "unwrap_or" | "unwrap_or_else"
                    | "unwrap_or_default" | "into" | "into_inner" | "into_iter" | "take"
                    | "ok" | "err" | "map_err" | "and_then" | "or_else"
            ) || call.method.to_string().starts_with("into_");
        };
        matches!(
            tc.registry.method_self_kind(&found),
            Some(crate::types::SelfKind::Value)
        )
    }

    fn consumes_scrutinee(&self, m: &syn::ExprMatch) -> bool {
        self.match_takes(m) == ownership::scrutinee::Takes::Payload
    }

    fn consumes_let_scrutinee(&self, let_expr: &syn::ExprLet) -> bool {
        self.let_takes(let_expr) == ownership::scrutinee::Takes::Payload
    }

    fn consumes_operands(&self, bin: &syn::ExprBinary) -> (bool, bool) {
        self.operator_takes(bin)
    }

    fn consumes_unary_operand(&self, unary: &syn::ExprUnary) -> bool {
        self.unary_takes(unary)
    }

    fn consumes_callee(&self, call: &syn::ExprCall) -> bool {
        // A callee whose bound is `FnOnce` and nothing else is consumed by the
        // call itself — `invoke` is the helper that says so. The move scan was
        // never told, so the parameter counted as untouched, and the body was
        // given no release at all in case the call had taken it: a path that
        // did not call left the closure and everything it captured to nobody.
        if self.bound_closure_helper(&call.func) == Some("invoke") {
            return true;
        }
        let syn::Expr::Path(path) = call.func.as_ref() else { return false };
        let Some(ident) = path.path.get_ident() else { return false };
        let name = crate::name_map::escape_reserved(&crate::name_map::to_camel_case(&ident.to_string()));
        self.own.once_closure_locals.borrow().iter().any(|n| *n == name)
    }
}
