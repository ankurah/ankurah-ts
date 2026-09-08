//! What a free call hands over that the SOURCE did not write.
//!
//! Split out of `dictionary.rs`, which is the conversion-dictionary machinery.
//! The one other synthetic argument the port appends is a MODE: the port's
//! `toValue` releases what it was handed or leaves it alone, and only the call
//! site knows which — `serde_json::to_value` takes its argument by value, and
//! the corpus's one site hands it a `&T` (HH4).

use crate::body::BodyTranslator;
use crate::ty::Ty;

impl BodyTranslator<'_> {
    /// HH4: `serde_json::to_value` takes its argument BY VALUE, and the port's
    /// own `toValue` needs telling which side of that a call site is on: where
    /// the source handed the value over, the JSON document is all that is left
    /// of it and the helper releases it; where the source wrote `&T` — which is
    /// what `core/src/value/mod.rs`'s `Value::json` writes, the one corpus site
    /// — the caller still owns it. Read off the WRITTEN argument, and the callee
    /// is identified by the canonical path the registry resolves its name to,
    /// never by the spelling the emitter chose (S4/I3).
    pub(crate) fn with_to_value_mode(
        &self,
        args: Vec<String>,
        call: &syn::ExprCall,
    ) -> Vec<String> {
        let mut args = args;
        if args.len() != 1 || !self.names_a_free_function(call, &["serde_json", "to_value"]) {
            return args;
        }
        let Some(written) = call.args.first() else { return args };
        // The TYPE decides, not the spelling: `to_value(h)` on an `h: &Held`
        // hands over a reference with no `&` written at the site.
        let Ok(ty) = self.quietly(|| self.resolve_expr_type(written)) else {
            self.fallback(
                syn::spanned::Spanned::span(written),
                "`serde_json::to_value` takes its argument by value and the engine could not \
                 type this one, so whether the caller still owns it is not known here; the \
                 borrowing mode is written, which leaks rather than releasing what somebody \
                 else holds",
            );
            return args;
        };
        if !matches!(ty, Ty::Ref { .. }) {
            args.push("'own'".to_string());
        }
        args
    }

    /// Does this call name the free function at this canonical path?
    fn names_a_free_function(&self, call: &syn::ExprCall, path: &[&str]) -> bool {
        let syn::Expr::Path(callee) = &*call.func else { return false };
        let Some(tc) = &self.types else { return false };
        let tc = tc.borrow();
        let segments: Vec<String> =
            callee.path.segments.iter().map(|s| s.ident.to_string()).collect();
        tc.registry.canonical_path(tc.module, &segments) == path
    }
}
