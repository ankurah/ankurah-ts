//! A call whose callee the engine could not resolve, and the free functions an
//! impl with no class of its own became.
//!
//! For: a method call is written from what the engine says the receiver IS, not
//! from the method's name — so when the engine cannot say, there is a second
//! table of the calls that mean the same thing whatever they are called on, and
//! this is where it is asked. And an impl written for a type with no emitted
//! class is a module-level function taking its receiver first, which is a
//! different call shape from a method on a class.

use crate::native_types;

use super::BodyTranslator;

impl BodyTranslator<'_> {
    // ── Method call translation ─────────────────────────────────────
    //
    // Dispatches to native_types modules based on resolved receiver type.
    // System types (Arc, RwLock, Result, etc.) pass through — their TS
    // implementations handle the method names directly.

    /// A call the engine could not resolve to a function.
    ///
    /// This is the transitional path spec section 4.11 keeps: the diagnostic has
    /// already been filed, and the translator does what it did before the impl
    /// table existed — reach through one wrapper if the receiver has one and the
    /// name is not its own, dispatch on whatever type it does know, and fall
    /// back to the name when it knows nothing. The std-surface step is what
    /// empties this path out; the fail-loud step deletes it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn translate_unresolved_call_using(&self, receiver: &str, rust_method: &str, ts_method: &str, args: &[String], receiver_expr: Option<&syn::Expr>, used: bool) -> String {
        if let (Some(receiver_expr), Some(tc)) = (receiver_expr, &self.types) {
            let tc_ref = tc.borrow();
            if let Ok(receiver_ty) = tc_ref.resolve_expr(receiver_expr) {
                let probe = tc_ref.probe();
                let step = probe.deref_once(&receiver_ty);
                let reach_through = !probe.declares_method(&receiver_ty, rust_method);
                let (target, receiver) = match (&step, reach_through) {
                    (Some(step), true) => {
                        let written = match &step.accessor {
                            Some(accessor) => format!("{}.{}", receiver, accessor.written()),
                            None => receiver.to_string(),
                        };
                        (step.to.clone(), written)
                    }
                    _ => (receiver_ty.clone(), receiver.to_string()),
                };
                let translated = native_types::translate_method_using(
                    tc_ref.registry, &target, &receiver, rust_method, args, used,
                );
                drop(tc_ref);
                return self.render_translation(
                    translated,
                    &receiver,
                    ts_method,
                    args,
                    syn::spanned::Spanned::span(receiver_expr),
                );
            }
        }

        // No type at all — the methods that translate the same way whatever the
        // receiver is.
        self.render_translation(
            native_types::translate_untyped(receiver, rust_method, args),
            receiver,
            ts_method,
            args,
            proc_macro2::Span::call_site(),
        )
    }

    // ── Function call translation ───────────────────────────────────
    //
    // Language-level constructs (Self, Ok/Err/Some/None, enum variants,
    // constructor heuristic) stay here. Type-specific translations
    // (Vec::new, HashMap::new, etc.) are in native_types/ modules.

    /// Does the type part of this callee path name a type this crate declared?
    ///
    /// `Vec::new` in ankurah is std's `Vec` unless ankurah declares one of its
    /// own; the name tables downstream cannot tell the difference, so the
    /// registry is asked before they are consulted.
    pub(crate) fn names_crate_type(&self, callee: Option<&syn::Path>) -> bool {
        let (Some(path), Some(tc)) = (callee, &self.types) else {
            return false;
        };
        if path.segments.len() < 2 {
            return false;
        }
        let owner: Vec<String> = path
            .segments
            .iter()
            .take(path.segments.len() - 1)
            .map(|s| s.ident.to_string())
            .collect();
        let tc = tc.borrow();
        matches!(
            tc.registry.lookup_type(tc.module, &owner),
            Ok(Some(crate::registry::Def::Type(id)))
                if !id.is_foreign() && !tc.registry.is_system(id)
        )
    }

}
