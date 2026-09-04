//! Reading an associated type through the impl that supplies it.
//!
//! Split out of `method.rs`, which had grown past the point where a reader
//! could hold it. The `Probe` is one object with one lifetime; these are its
//! other two jobs, each in its own file.

use super::impls::{head_of, ImplId};
use super::method::MAX_BOUND_DEPTH;
use super::Probe;
use crate::ty::{TraitRef, Ty};
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
            other => other.clone(),
        }
    }

    /// The bound on a `dyn Trait` or a bounded parameter that declares this
    /// associated name.
    pub(super) fn declaring_bound(&self, base: &Ty, name: &str) -> Option<TraitRef> {
        let bounds: Vec<TraitRef> = match base {
            Ty::Dyn { traits } | Ty::ImplTrait { bounds: traits } => traits.clone(),
            Ty::Param(param) => self
                .param_bounds
                .iter()
                .filter(|(p, _)| p == param)
                .map(|(_, t)| t.clone())
                .collect(),
            _ => return None,
        };
        bounds.into_iter().find(|b| {
            self.reg
                .trait_def(b.id)
                .is_some_and(|d| d.assoc_types.iter().any(|a| a == name))
        })
    }

    /// The type an impl supplies for one associated name.
    pub(super) fn project(&self, base: &Ty, trait_: Option<&TraitRef>, name: &str) -> Option<Ty> {
        // A projection on a trait object or on a bounded parameter is answered
        // by whichever bound declares the name — `Self::Item` inside a trait's
        // own default body means that trait's `Item`.
        if let Some(bound) = self.declaring_bound(base, name) {
            if let Some(bound_ty) = bound.bindings.iter().find(|(n, _)| n == name) {
                return Some(bound_ty.1.clone());
            }
            // The trait declares it but the use site did not bind it, so there
            // is no type to give: leaving the projection standing says so.
            return None;
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
