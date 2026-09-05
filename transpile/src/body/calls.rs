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
                let bind_receiver = |written: &str| self.name_once(Some(receiver_expr), written);
                let bind_eager = |_: usize, written: &str| Some(written.to_string());
                let once = native_types::nullable::Once {
                    bind_receiver: &bind_receiver,
                    bind_eager: &bind_eager,
                };
                let translated = native_types::translate_method_using(
                    tc_ref.registry, &target, &receiver, rust_method, args, used, &once,
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

// ── Reading a value once ────────────────────────────────────────────
//
// An `Option` combinator is written as the test it is, and a test reads the
// value it tests and then reads it AGAIN to hand it on. Rust reads it once. So
// the receiver is given one name before the test, unless reading it twice is
// the same as reading it once.

impl BodyTranslator<'_> {
    /// One name for a value a translation reads twice.
    ///
    /// A place comes back as it is: reading a name, a field of one or an index
    /// into one reads the same storage again and runs nothing, which is the
    /// same reason Rust may read it once. Anything else is named before the
    /// statement it stands in, and both reads read that name.
    pub(crate) fn name_once(&self, expr: Option<&syn::Expr>, written: &str) -> String {
        match expr {
            Some(expr) if reads_the_same_twice(expr) => written.to_string(),
            // With no expression to ask about there is nothing to decide from,
            // and a value read twice is the defect: the name is taken.
            _ => self.hoist_name(written.to_string()),
        }
    }

    /// A value Rust evaluates BEFORE it branches — `ok_or`'s error, `map_or`'s
    /// default — where the port can name it.
    ///
    /// Rust builds such a value on both paths and drops it on the one that does
    /// not use it. Naming it here restores the order; it does not restore the
    /// drop, because there is nothing at this point to release it under. So a
    /// value that owns something stays inside the branch and the difference is
    /// said out loud, rather than being built on both paths and leaked on one.
    pub(crate) fn name_eager(&self, expr: Option<&syn::Expr>, written: &str) -> Option<String> {
        let expr = expr?;
        if reads_the_same_twice(expr) {
            return Some(written.to_string());
        }
        if self.owns_something(expr) {
            self.fallback(
                syn::spanned::Spanned::span(expr),
                "Rust evaluates this argument before it branches and drops it on the path that \
                 does not use it; the port has no name here to drop it under, so it is \
                 evaluated inside the branch that uses it instead",
            );
            return None;
        }
        Some(self.hoist_name(written.to_string()))
    }

    /// Does this expression produce something the runtime releases?
    fn owns_something(&self, expr: &syn::Expr) -> bool {
        let Some(tc) = &self.types else { return true };
        let tc = tc.borrow();
        let Ok(ty) = tc.resolve_expr(expr) else { return true };
        crate::ownership::drops_of(&tc.probe(), &ty).is_droppable()
    }
}

/// Can this expression be written twice without the program noticing?
///
/// A name, a field of one, an index into one, a literal: reading it again reads
/// the same storage and runs nothing. A call, a `?`, an `await`, an operator —
/// anything that DOES something — is written once and read through a name.
/// `is_place` answers a different question (what a statement has to release) and
/// counts `x?` as a place, which reading twice would unwrap twice.
fn reads_the_same_twice(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Path(_) | syn::Expr::Lit(_) => true,
        syn::Expr::Field(field) => reads_the_same_twice(&field.base),
        syn::Expr::Index(index) => {
            reads_the_same_twice(&index.expr) && reads_the_same_twice(&index.index)
        }
        syn::Expr::Unary(unary) => {
            matches!(unary.op, syn::UnOp::Deref(_)) && reads_the_same_twice(&unary.expr)
        }
        syn::Expr::Reference(r) => reads_the_same_twice(&r.expr),
        syn::Expr::Paren(p) => reads_the_same_twice(&p.expr),
        syn::Expr::Group(g) => reads_the_same_twice(&g.expr),
        // A JavaScript value is neither borrowed nor owned, so the four `as_`
        // conversions between those states are the value itself — the port
        // writes `self.order_by.as_ref()` as `this.orderBy`, which is the place
        // it started as.
        syn::Expr::MethodCall(call) if call.args.is_empty() => {
            matches!(call.method.to_string().as_str(), "as_ref" | "as_mut" | "as_deref" | "as_deref_mut")
                && reads_the_same_twice(&call.receiver)
        }
        _ => false,
    }
}
