//! What a resolved call's callee declares: how it takes its receiver, what it
//! takes at each parameter, and the parameters and bounds of its own.
//!
//! Split out of `method.rs`, which was over the 600-line rule. Nothing here
//! decides WHICH function a call lands on — that is the probe above — only what
//! the one it landed on says about itself.

use super::{Callee, MethodResolution};
use crate::registry::{MethodSig, TypeRegistry};
use crate::ty::subst::Subst;
use crate::ty::{TraitRef, Ty};
use crate::types::SelfKind;

impl TypeRegistry {
    /// How a resolved call takes its receiver.
    ///
    /// The ownership emission turns on it: a method declared `self` takes the
    /// receiver with it, so the scope that held the receiver no longer owns it
    /// and must not release it.
    pub fn method_self_kind(&self, found: &MethodResolution) -> Option<SelfKind> {
        match &found.callee {
            Callee::Inherent(id, name)
            | Callee::TraitImpl(id, name)
            | Callee::Blanket(id, name) => {
                let def = self.impl_def(*id);
                if let Some(sig) = def.methods.get(name) {
                    return sig.self_kind;
                }
                // An impl that inherited the trait's default body has no
                // signature of its own; the trait's declaration is the answer.
                let trait_id = def.trait_ref.as_ref()?.id;
                self.trait_method(trait_id, name)?.1.sig.self_kind
            }
            Callee::TraitObject(trait_id, name, _) => {
                self.trait_method(*trait_id, name)?.1.sig.self_kind
            }
        }
    }

    /// The signature the callee was resolved to: what it declares, what it
    /// answers, and the parameters and bounds of its own that a caller has to
    /// satisfy.
    pub fn method_sig(&self, found: &MethodResolution) -> Option<MethodSig> {
        match &found.callee {
            Callee::Inherent(id, name)
            | Callee::TraitImpl(id, name)
            | Callee::Blanket(id, name) => {
                let def = self.impl_def(*id);
                match def.methods.get(name) {
                    Some(sig) => Some(sig.clone()),
                    // An impl that inherited the trait's default body has no
                    // signature of its own; the trait's declaration is it.
                    None => def
                        .trait_ref
                        .as_ref()
                        .and_then(|t| self.trait_method(t.id, name))
                        .map(|(_, m)| m.sig.clone()),
                }
            }
            Callee::TraitObject(trait_id, name, _) => {
                self.trait_method(*trait_id, name).map(|(_, m)| m.sig.clone())
            }
        }
    }

    /// What the resolved callee declares each argument to be, in order, with
    /// the impl's parameters already bound to what stood at their positions.
    ///
    /// This is where a closure argument gets its parameter types (spec 4.5) and
    /// where an `.into()` in argument position learns what it converts to
    /// (spec 4.6): the argument's type is not in the argument, it is in the
    /// signature the call resolved to.
    pub fn method_param_types(&self, found: &MethodResolution) -> Vec<Ty> {
        let sig = self.method_sig(found);
        // `Iterator::map` declares `F: FnMut(Self::Item) -> B`, and a
        // resolution against an impl binds that impl's parameters without ever
        // naming `Self` — the receiver is what `Self` is, so it is put in here
        // for the projections in the bound to settle against.
        let mut subst = found.subst.clone();
        subst
            .entry("Self".to_string())
            .or_insert_with(|| found.adjusted.peel_refs().clone());

        sig.map(|sig| {
            sig.params
                .iter()
                .map(|(_, ty)| with_bounds(&sig, &ty.substitute(&subst), &subst))
                .collect()
        })
        .unwrap_or_default()
    }
}

/// A parameter whose type is one of the method's own type parameters, rewritten
/// as the `impl Trait` it stands for.
///
/// `fn map<B, F: FnMut(Self::Item) -> B>(self, f: F)` says what `f` can do in
/// the `where` clause, not in `f`'s type, and Rust treats that as identical to
/// writing `f: impl FnMut(Self::Item) -> B`. A closure passed there takes its
/// parameter types from the bound, so the bound has to travel with the
/// parameter.
fn with_bounds(sig: &MethodSig, ty: &Ty, subst: &Subst) -> Ty {
    let Ty::Param(name) = ty else {
        return ty.clone();
    };
    if !sig.type_params.iter().any(|p| p == name) {
        return ty.clone();
    }
    let bounds: Vec<TraitRef> = sig
        .bounds
        .iter()
        .filter(|b| matches!(&b.subject, Ty::Param(subject) if subject.as_str() == name.as_str()))
        .map(|b| b.trait_ref.substitute(subst))
        .collect();
    if bounds.is_empty() {
        ty.clone()
    } else {
        Ty::ImplTrait { bounds }
    }
}
