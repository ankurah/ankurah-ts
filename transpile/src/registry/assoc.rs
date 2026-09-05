//! Reading an associated type through the impl that supplies it.
//!
//! Split out of `method.rs`, which had grown past the point where a reader
//! could hold it. The `Probe` is one object with one lifetime; these are its
//! other two jobs, each in its own file.

use super::impls::{head_of, ImplId};
use super::method::MAX_BOUND_DEPTH;
use super::Probe;
use crate::ty::{TraitRef, Ty};

/// What a bound says about an associated name it declares.
pub(super) enum BoundAssoc {
    /// The site wrote what the name is: `DerefMut<Target = BTreeMap<K, V>>`.
    Bound(Ty),
    /// Something in the bound's chain declares the name and nothing here says
    /// what it is.
    Open,
}
impl Probe<'_> {
    // ── Associated types ───────────────────────────────────────────────

    /// Replace every projection the impl table can answer. `<Vec<u8> as
    /// TryInto<Clock>>::Error` becomes whatever that impl wrote for `Error`;
    /// a projection no impl supplies is left standing, which is the truth
    /// about it.
    pub fn normalize(&self, ty: &Ty) -> Ty {
        self.normalize_within(ty, 0)
    }

    pub(super) fn normalize_within(&self, ty: &Ty, depth: usize) -> Ty {
        if depth >= MAX_BOUND_DEPTH {
            return ty.clone();
        }
        match ty {
            Ty::Assoc { base, trait_, name } => {
                let base = self.normalize_within(base, depth + 1);
                match self.project(&base, trait_.as_deref(), name) {
                    Some(found) => self.normalize_within(&found, depth + 1),
                    None => Ty::Assoc {
                        base: Box::new(base),
                        trait_: trait_.clone(),
                        name: name.clone(),
                    },
                }
            }
            Ty::Named { id, args } => Ty::Named {
                id: *id,
                args: args
                    .iter()
                    .map(|a| self.normalize_within(a, depth + 1))
                    .collect(),
            },
            Ty::Ref { mutable, inner } => Ty::Ref {
                mutable: *mutable,
                inner: Box::new(self.normalize_within(inner, depth + 1)),
            },
            Ty::Tuple(elems) => Ty::Tuple(
                elems
                    .iter()
                    .map(|e| self.normalize_within(e, depth + 1))
                    .collect(),
            ),
            Ty::Slice(inner) => Ty::Slice(Box::new(self.normalize_within(inner, depth + 1))),
            Ty::Array { elem, len } => Ty::Array {
                elem: Box::new(self.normalize_within(elem, depth + 1)),
                len: len.clone(),
            },
            // A bound carries types too, and the one that matters most is
            // `FnMut(Self::Item)`: the element type a closure's parameter takes
            // is written inside the bound, and leaving it as a projection there
            // handed the closure a type nothing resolves against.
            Ty::ImplTrait { bounds } => Ty::ImplTrait {
                bounds: self.normalize_bounds(bounds, depth),
            },
            Ty::Dyn { traits } => Ty::Dyn {
                traits: self.normalize_bounds(traits, depth),
            },
            other => other.clone(),
        }
    }

    fn normalize_bounds(&self, bounds: &[TraitRef], depth: usize) -> Vec<TraitRef> {
        bounds
            .iter()
            .map(|bound| TraitRef {
                id: bound.id,
                args: bound
                    .args
                    .iter()
                    .map(|a| self.normalize_within(a, depth + 1))
                    .collect(),
                bindings: bound
                    .bindings
                    .iter()
                    .map(|(name, ty)| (name.clone(), self.normalize_within(ty, depth + 1)))
                    .collect(),
            })
            .collect()
    }

    /// The traits a `dyn Trait`, an `impl Trait` or a bounded parameter is
    /// required to implement here.
    fn bounds_of(&self, base: &Ty) -> Vec<TraitRef> {
        match base {
            Ty::Dyn { traits } | Ty::ImplTrait { bounds: traits } => traits.clone(),
            Ty::Param(param) => self
                .param_bounds
                .iter()
                .filter(|(p, _)| p == param)
                .map(|(_, t)| t.clone())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// What a bound on a `dyn Trait` or a bounded parameter says about one
    /// associated name.
    ///
    /// The name a bound answers for need not be declared by the trait the site
    /// wrote: `impl DerefMut<Target = BTreeMap<K, V>>` writes `Target` on
    /// `DerefMut`, which declares no associated type of its own — `Deref`
    /// declares it, and Rust resolves the binding through the supertrait.
    /// Asking only the written trait left `<impl DerefMut>::Target` standing,
    /// and every call on the map behind it was written from its name alone.
    pub(super) fn bound_assoc(&self, base: &Ty, name: &str) -> Option<BoundAssoc> {
        self.bounds_of(base)
            .iter()
            .find_map(|bound| self.assoc_through(bound, name, &mut Vec::new()))
    }

    fn assoc_through(
        &self,
        of: &TraitRef,
        name: &str,
        seen: &mut Vec<crate::ty::TypeId>,
    ) -> Option<BoundAssoc> {
        if seen.contains(&of.id) {
            return None;
        }
        seen.push(of.id);
        // A binding written at the site answers first, wherever in the chain
        // the name is declared — that is what the site wrote it to say.
        if let Some((_, ty)) = of.bindings.iter().find(|(n, _)| n == name) {
            return Some(BoundAssoc::Bound(ty.clone()));
        }
        let def = self.reg.trait_def(of.id)?;
        let declares = def.assoc_types.iter().any(|a| a == name);
        // The supertraits are written in terms of this trait's parameters, so
        // they are instantiated with what stood at them before the search goes
        // on — the same walk `trait_method_of` takes for a method.
        let mut subst = crate::ty::bind_params(&def.generics, &of.args);
        for (assoc, ty) in &of.bindings {
            subst.insert(assoc.clone(), ty.clone());
        }
        let supers: Vec<TraitRef> = def
            .supertraits
            .iter()
            .map(|t| t.substitute(&subst))
            .collect();
        for supertrait in &supers {
            if let Some(found) = self.assoc_through(supertrait, name, seen) {
                return Some(found);
            }
        }
        declares.then_some(BoundAssoc::Open)
    }

    /// The type an impl supplies for one associated name.
    pub(super) fn project(&self, base: &Ty, trait_: Option<&TraitRef>, name: &str) -> Option<Ty> {
        // A projection on a trait object or on a bounded parameter is answered
        // by whichever bound declares the name — `Self::Item` inside a trait's
        // own default body means that trait's `Item`.
        match self.bound_assoc(base, name) {
            Some(BoundAssoc::Bound(ty)) => return Some(ty),
            // The trait declares it but the use site did not bind it, so there
            // is no type to give: leaving the projection standing says so.
            Some(BoundAssoc::Open) => return None,
            None => {}
        }
        let mut found: Option<Ty> = None;
        let ids: Vec<ImplId> = match trait_ {
            Some(tr) => self.reg.impls().of_trait(tr.id).to_vec(),
            None => self
                .reg
                .impls()
                .for_head(&head_of(base, &[]))
                .collect::<Vec<_>>(),
        };
        for id in ids {
            let def = self.reg.impl_def(id);
            let Some(assoc) = def.assoc_types.get(name) else {
                continue;
            };
            let Some(mut subst) = def.match_self(base) else {
                continue;
            };
            if let Some(tr) = trait_ {
                // The projection names the trait *with its arguments*, and those
                // arguments can be what fixes the impl's own parameters:
                // `impl<T, I: SliceIndex<[T]>> Index<I> for Vec<T>` learns `I`
                // from the `Index<usize>` the site asked for. Comparing the two
                // for equality instead left `I` open and the impl unusable.
                let Some(implemented) = def.trait_ref.as_ref() else {
                    continue;
                };
                let Some(from_args) = def.match_written_args(implemented, tr) else {
                    continue;
                };
                for (param, ty) in from_args {
                    subst.entry(param).or_insert(ty);
                }
            }
            self.infer_from_bounds(def, &mut subst);
            // `impl<I: Iterator> IntoIterator for I` matches every base there
            // is, and supplies `Item = <I as Iterator>::Item`. Without its bound
            // checked, a `Vec<u8>` had two answers for `IntoIterator::Item` —
            // the real `u8` and that unnormalisable projection — and two
            // different answers is no answer.
            match self.bounds_hold(&def.bounds, &subst) {
                Some(deferred) if deferred.is_empty() => {}
                _ => continue,
            }
            if let Some(tr) = trait_ {
                // `<S as Carrier<u8>>::Item` is still not what
                // `impl Carrier<u16> for S` supplies: the arguments have to
                // agree once everything the match learned is filled in.
                let Some(impl_trait) = def.trait_ref.as_ref() else {
                    continue;
                };
                if impl_trait.substitute(&subst).args != tr.args {
                    continue;
                }
            }
            let projected = assoc.substitute(&subst);
            match &found {
                // Two impls supplying different answers is not something to pick
                // between; leave the projection standing and let the caller say
                // it could not be read.
                Some(existing) if *existing != projected => return None,
                _ => found = Some(projected),
            }
        }
        found
    }}
