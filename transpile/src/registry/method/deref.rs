//! Every receiver a written one reaches, and what is written to reach it.
//!
//! Rust tries the receiver itself, then each dereference, then the unsized form
//! of the last, and calls the first method that answers. That a type
//! dereferences is a fact from the impl table; how the hop is WRITTEN is a fact
//! about the port's runtime.

use super::{Accessor, DerefKind, DerefStep, MethodError, Obligation, Probe, MAX_DEREF_STEPS};
use crate::ty::Ty;

impl Probe<'_> {

    /// One hop: `&T` to `T`, or through the `Deref` impl written for it.
    pub fn deref_once(&self, ty: &Ty) -> Option<DerefStep> {
        self.deref_once_reporting(ty, &mut Vec::new())
    }

    /// The same, collecting the bounds that stopped a conditional `Deref` from
    /// being taken, so the caller can say why the type behind it was not
    /// reached instead of silently stopping the chain.
    pub(super) fn deref_once_reporting(&self, ty: &Ty, undecided: &mut Vec<Obligation>) -> Option<DerefStep> {
        if let Ty::Ref { inner, .. } = ty {
            return Some(DerefStep {
                from: ty.clone(),
                to: (**inner).clone(),
                kind: DerefKind::Builtin,
                accessor: None,
            });
        }
        let deref = self.reg.deref_trait()?;
        for &id in self.reg.impls().of_trait(deref) {
            let def = self.reg.impl_def(id);
            let Some(mut subst) = def.match_self(ty) else {
                continue;
            };
            self.infer_from_bounds(def, &mut subst);
            // A conditional `impl<T: Bound> Deref for Wrapper<T>` does not
            // dereference a `Wrapper<NoBound>`, and one whose bound nobody can
            // decide does not dereference anything either: taking the step would
            // be guessing at the type behind it.
            match self.bounds_hold(&def.bounds, &subst) {
                Some(deferred) if deferred.is_empty() => {}
                Some(deferred) => {
                    undecided.extend(deferred);
                    continue;
                }
                None => continue,
            }
            let Some(target) = def.assoc_types.get("Target") else {
                continue;
            };
            return Some(DerefStep {
                from: ty.clone(),
                to: self.normalize(&target.substitute(&subst)),
                kind: DerefKind::Overloaded(id),
                accessor: self.step_accessor(ty),
            });
        }
        None
    }

    /// What has to be written to reach through one `Deref` step.
    ///
    /// That a type dereferences at all is a Rust fact and comes from the impl
    /// table; how the hop is *written* is a fact about the port's runtime and
    /// comes from `name_map::system_shapes`, keyed by the type's identity. An
    /// `Arc` keeps its value in `.value`, a `Box` is its value, and a crate's own
    /// `impl Deref` is a function the emitted class carries — Rust inserts that
    /// call, and so must the TypeScript, or the field behind the wrapper is read
    /// off the wrapper.
    fn step_accessor(&self, ty: &Ty) -> Option<Accessor> {
        let Some(id) = ty.id() else {
            return Some(Accessor::Call("deref".to_string()));
        };
        if !self.reg.is_system(id) {
            return Some(Accessor::Call("deref".to_string()));
        }
        match self.reg.shapes().accessor(id) {
            Some(crate::name_map::system_shapes::Accessor::Field(name)) => {
                Some(Accessor::Field(name.to_string()))
            }
            Some(crate::name_map::system_shapes::Accessor::Transparent) => None,
            // A declared std type the port does not wrap — a lock, an iterator
            // adaptor — dereferences without anything being written for it.
            None => None,
        }
    }

    /// Every receiver reachable from the written one, in the order Rust tries
    /// them: itself, then each dereference, then the unsized form of the last.
    pub fn deref_chain(&self, receiver: &Ty) -> Result<Vec<DerefStep>, MethodError> {
        self.deref_chain_reporting(receiver, &mut Vec::new())
    }

    pub(super) fn deref_chain_reporting(
        &self,
        receiver: &Ty,
        undecided: &mut Vec<Obligation>,
    ) -> Result<Vec<DerefStep>, MethodError> {
        let mut steps: Vec<DerefStep> = Vec::new();
        let mut current = receiver.clone();
        while let Some(step) = self.deref_once_reporting(&current, undecided) {
            if steps.len() >= MAX_DEREF_STEPS {
                return Err(MethodError::DerefCycle {
                    receiver: receiver.clone(),
                });
            }
            current = step.to.clone();
            steps.push(step);
        }
        // `[T; N]` becomes `[T]` at the end of the chain, which is the only
        // unsizing a receiver in this corpus needs.
        if let Ty::Array { elem, .. } = &current {
            steps.push(DerefStep {
                from: current.clone(),
                to: Ty::Slice(elem.clone()),
                kind: DerefKind::Unsize,
                accessor: None,
            });
        }
        Ok(steps)
    }
}
