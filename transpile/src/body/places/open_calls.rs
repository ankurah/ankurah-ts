//! A call whose IMPL is chosen at run time, because the bound is open.
//!
//! For: a method called on a type parameter resolves only to the blanket impl
//! or to the trait's own declaration, and Rust picks the real impl per
//! instantiation. One emitted body cannot, so the port writes a dispatcher that
//! tests the receiver's SHAPE and calls the impl's own function. Where no
//! dispatcher can be written — two impls the run time cannot tell apart, or a
//! trait declared in another crate, which carries its dispatcher there — the
//! site says so instead of naming a function nothing declares.

use super::super::BodyTranslator;

impl<'a> BodyTranslator<'a> {
    /// The function that picks among a trait's impls at run time, for a call
    /// the engine resolved only to the blanket one.
    ///
    /// The blanket impl is what the engine picks when the bound is open, and it
    /// is right only for the receivers the blanket is written for. The
    /// dispatcher tests the receiver's shape instead, so every impl of the
    /// trait is reachable — and where no dispatcher can be written, the site
    /// says which impls the emitted call cannot reach.
    pub(crate) fn open_dispatcher(
        &self,
        free: &crate::emit_impls::FreeCall,
        call: &syn::ExprMethodCall,
    ) -> Option<String> {
        let tc = self.types.as_ref()?;
        let tc = tc.borrow();
        let reg = tc.registry;
        let trait_ref = reg.impl_def(free.impl_id).trait_ref.as_ref()?;
        let trait_id = trait_ref.id;
        let trait_name = reg.name_of(trait_id);
        // A trait another crate declares carries its dispatcher there, and this
        // run does not read that crate.
        let declared_here = reg
            .def(trait_id)
            .is_some_and(|def| !reg.modules().get(def.module).is_system);
        let refused = if declared_here {
            crate::emit_impls::dispatcher_refusal(reg, trait_id, &trait_name, &call.method.to_string())
        } else {
            Some("the trait is declared outside this crate, where its dispatcher lives".to_string())
        };
        if let Some(why) = refused {
            drop(tc);
            self.fallback(
                syn::spanned::Spanned::span(call),
                format!(
                    "`{}` here dispatches through a bound the engine cannot close, and no \
                     run-time selection among the trait's impls can be written because {}; the \
                     call is written as `{}`, the blanket impl's function, and a receiver one \
                     of the trait's other impls is written for reaches the wrong one",
                    call.method, why, free.name
                ),
            );
            return None;
        }
        crate::emit_impls::record_wanted(trait_id, &call.method.to_string());
        Some(crate::emit_impls::dispatcher_name(
            &trait_name,
            &crate::name_map::map_fn_name(&call.method.to_string()),
        ))
    }

    /// The name a call through an OPEN BOUND has to write, where the trait's
    /// impls become module-level functions.
    ///
    /// The engine resolved `subject.members()` only to `TClock`'s declaration,
    /// because `subject` is a type parameter. `Clock`'s impl of that trait is
    /// written in core while `Clock` itself is declared in proto, so the method
    /// is `Clock_members(self)` and the class carries nothing called `members`.
    /// One such impl is called by name; several go through the dispatcher.
    pub(crate) fn open_bound_call(
        &self,
        tc: &crate::infer::TypeContext<'_>,
        found: &crate::registry::MethodResolution,
        call: &syn::ExprMethodCall,
    ) -> Option<String> {
        let crate::registry::Callee::TraitObject(trait_id, method) = &found.callee else {
            return None;
        };
        let reg = tc.registry;
        let trait_name = reg.name_of(*trait_id);
        // A trait declared OUTSIDE this crate carries its dispatcher wherever
        // it was declared, and this run does not write one — so naming it here
        // writes a call to a function nothing declares. The same test the
        // blanket-impl path has made: `x.try_into()` on a bounded parameter
        // reached `TryInto`'s declaration once a declared bound could beat an
        // undecided blanket, and wrote `TryInto_dispatch_tryInto(..)` at five
        // sites in core and storage-indexeddb.
        let declared_here = reg
            .def(*trait_id)
            .is_some_and(|def| !reg.modules().get(def.module).is_system);
        match crate::emit_impls::open_bound_call(reg, *trait_id, &trait_name, method)? {
            crate::emit_impls::OpenCall::One(name) => Some(name),
            crate::emit_impls::OpenCall::Dispatch => {
                let refused = match declared_here {
                    true => crate::emit_impls::dispatcher_refusal(reg, *trait_id, &trait_name, method),
                    false => Some(
                        "the trait is declared outside this crate, where its dispatcher lives"
                            .to_string(),
                    ),
                };
                if let Some(why) = refused {
                    self.fallback(
                        syn::spanned::Spanned::span(call),
                        format!(
                            "`{}` here is called through a bound on `{}`, whose impls are \
                             emitted as module-level functions, and no run-time selection \
                             among them can be written because {}; the call is written as a \
                             method on the receiver, which carries none",
                            method, trait_name, why
                        ),
                    );
                    return None;
                }
                crate::emit_impls::record_wanted(*trait_id, method);
                Some(crate::emit_impls::dispatcher_name(
                    &trait_name,
                    &crate::name_map::map_fn_name(method),
                ))
            }
        }
    }
}
