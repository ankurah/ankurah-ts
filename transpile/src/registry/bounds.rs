//! Whether a `where` clause holds, and what is left undecided.
//!
//! Split out of `method.rs`, which had grown past the point where a reader
//! could hold it. The `Probe` is one object with one lifetime; these are its
//! other two jobs, each in its own file.

use super::impls::{head_of, Bound};
use super::method::{open_params, Holds, Obligation, Undecided, MAX_BOUND_DEPTH, SIZED_PATH};
use super::Probe;
use crate::ty::subst::Subst;
use crate::ty::{TraitRef, Ty, TypeId};
impl Probe<'_> {
    // ── Bounds ─────────────────────────────────────────────────────────

    /// Do this impl's `where` clauses hold for the types just bound? `None` when
    /// one of them definitely does not; otherwise the ones the engine could not
    /// decide, travelling with the answer.
    pub(super) fn bounds_hold(&self, bounds: &[Bound], subst: &Subst) -> Option<Vec<Obligation>> {
        let mut deferred = Vec::new();
        for bound in bounds {
            // A bound's parts can be projections — `B: FromIterator<Self::Item>`
            // — and a projection matches nothing until the impl table has read
            // it through. Deciding one unread reported `Vec<u8>` as not
            // implementing `FromIterator<u8>`.
            let subject = self.normalize(&bound.subject.substitute(subst));
            let trait_ref = self.normalize_trait_ref(&bound.trait_ref.substitute(subst));
            match self.holds(&subject, &trait_ref, 0) {
                Holds::Yes => {}
                Holds::No => return None,
                Holds::Undecided(reason) => deferred.push(Obligation {
                    subject,
                    bound: trait_ref,
                    reason,
                }),
            }
        }
        Some(deferred)
    }

    pub(super) fn holds(&self, subject: &Ty, trait_ref: &TraitRef, depth: usize) -> Holds {
        if depth >= MAX_BOUND_DEPTH {
            return Holds::Undecided(Undecided::DepthLimit);
        }
        // A trait nothing declares says nothing about the subject. `Fn(T)` is
        // the common one; the closure step decides those.
        let Some(declared) = self.reg.trait_def(trait_ref.id) else {
            return Holds::Undecided(Undecided::NoDeclaration);
        };
        // `Send`, `Sync`, `Unpin` and their kin are decided by rustc from the
        // shape of the type, not from an impl anyone wrote, so there is nothing
        // in the table to find. The corpus compiles, which means every
        // auto-trait bound in it already holds; searching for an impl would
        // only ever report an obligation that Rust had already discharged.
        if declared.is_auto {
            return Holds::Yes;
        }
        // `Sized` is not an auto trait and has no impl anywhere: rustc decides
        // it from the layout. Everything the corpus writes as a type argument
        // is sized — the unsized ones (`str`, `[T]`, `dyn Trait`) are only ever
        // written behind a reference — so a `T: Sized` bound holds, and
        // refusing it rejected every blanket written with one. A `?Sized`
        // bound is a *relaxation* and never reaches here at all.
        if self
            .reg
            .system_type(SIZED_PATH)
            .is_some_and(|sized| sized == trait_ref.id)
        {
            return Holds::Yes;
        }
        // A bound written on a parameter in scope is the proof: inside
        // `impl<SE: StorageEngine> Node<SE>`, `SE: StorageEngine` holds by
        // declaration and there is no impl to go looking for.
        if let Ty::Param(name) = subject {
            if self
                .param_bounds
                .iter()
                .any(|(p, t)| p == name && t == trait_ref)
            {
                return Holds::Yes;
            }
        }
        // A subject that is *itself* still open is not a type an impl can be
        // found for. One that merely holds an open argument can be: `impl<K, V>
        // Iterator for Values<K, V>` proves `Values<usize, Listener<T>>:
        // Iterator` whatever `T` turns out to be, and refusing to look made
        // every iterator chain inside a generic impl an open question.
        if matches!(subject, Ty::Param(_) | Ty::Infer | Ty::Assoc { .. }) {
            return Holds::Undecided(Undecided::OpenSubject);
        }
        // A trait object implements the traits it names, with the arguments it
        // names them with.
        if let Ty::Dyn { traits } | Ty::ImplTrait { bounds: traits } = subject {
            if traits.iter().any(|t| t == trait_ref) {
                return Holds::Yes;
            }
        }
        let mut undecided: Option<Undecided> = None;
        for &id in self.reg.impls().of_trait(trait_ref.id) {
            let def = self.reg.impl_def(id);
            let Some(mut subst) = def.match_self(subject) else {
                continue;
            };
            // The trait's own arguments have to agree: `impl Marker<u16> for S`
            // says nothing about `S: Marker<u8>`. They are *matched* rather than
            // compared, because an argument can be what fixes one of the impl's
            // own parameters — `unsafe impl<T> SliceIndex<[T]> for usize` learns
            // `T` from the `SliceIndex<[u8]>` the bound asked for.
            let Some(implemented) = def.trait_ref.as_ref() else {
                continue;
            };
            if implemented.id != trait_ref.id {
                continue;
            }
            let Some(from_args) = def.match_written_args(implemented, trait_ref) else {
                continue;
            };
            for (param, ty) in from_args {
                subst.entry(param).or_insert(ty);
            }
            self.infer_from_bounds(def, &mut subst);
            let implemented = implemented.substitute(&subst);
            if implemented.args != trait_ref.args {
                continue;
            }
            // An associated binding in a bound — the `Item = &T` in
            // `I: Iterator<Item = &'a T>` — is a constraint on what the impl
            // supplies, not another argument to the trait. Comparing it as one
            // made every such bound fail, and with it every iterator adaptor
            // whose own impl carries one.
            if !self.bindings_agree(&implemented, trait_ref, def, &subst) {
                continue;
            }
            let mut all = true;
            for inner in &def.bounds {
                match self.holds(
                    &inner.subject.substitute(&subst),
                    &inner.trait_ref.substitute(&subst),
                    depth + 1,
                ) {
                    Holds::Yes => {}
                    // An inner bound nobody can decide leaves the outer one
                    // undecided too. Dropping it here reported the whole impl as
                    // proven on the strength of a question nobody answered.
                    Holds::Undecided(reason) => undecided = Some(reason),
                    Holds::No => {
                        all = false;
                        break;
                    }
                }
            }
            if all {
                return match undecided {
                    Some(reason) => Holds::Undecided(reason),
                    None => Holds::Yes,
                };
            }
        }
        match undecided {
            Some(reason) => Holds::Undecided(reason),
            // Nothing in the table implements it. An open argument in the
            // subject leaves that a question only where some impl of the trait
            // is written for the same head, so that the argument could be why
            // the match failed. `HashMap<K, V, S>: Iterator` is `No` however
            // open `K` is, because no impl of `Iterator` is for a `HashMap`.
            None if subject.has_open_param() && self.head_is_implemented(subject, trait_ref) => {
                Holds::Undecided(Undecided::OpenSubject)
            }
            None => Holds::No,
        }
    }

    /// Could an impl of this trait ever be for a subject of this shape?
    pub(super) fn head_is_implemented(&self, subject: &Ty, trait_ref: &TraitRef) -> bool {
        let head = head_of(subject, &[]);
        self.reg.impls().of_trait(trait_ref.id).iter().any(|&id| {
            let def = self.reg.impl_def(id);
            def.is_blanket() || head_of(&def.self_ty, &def.generics) == head
        })
    }

    /// Is there an impl of this trait for this type, whatever its arguments?
    ///
    /// Weaker than `holds`, which compares the trait's arguments too. This is
    /// the question "is it an iterator at all", asked where the answer decides
    /// only how the value is written and not what it holds.
    pub fn implements(&self, ty: &Ty, trait_id: TypeId) -> bool {
        self.reg.impls().of_trait(trait_id).iter().any(|&id| {
            let def = self.reg.impl_def(id);
            !def.is_blanket()
                && def
                    .match_self(ty)
                    .is_some_and(|subst| self.bounds_hold(&def.bounds, &subst).is_some())
        })
    }

    /// Bind the impl parameters that only its own bounds mention.
    ///
    /// `impl<'a, T: Clone, I: Iterator<Item = &'a T>> Iterator for Cloned<I>`
    /// says `type Item = T`, and matching the self type against
    /// `Cloned<Values<'_, K, V>>` binds `I` and nothing else. `T` is fixed by
    /// the bound: `Values`'s own `Iterator::Item` is `&V`, and matching that
    /// against `&'a T` gives `T = V`. Without this the adaptor's `Item` came
    /// back as a loose parameter, which is not an answer.
    pub(super) fn infer_from_bounds(&self, def: &super::impls::ImplDef, subst: &mut Subst) {
        for bound in &def.bounds {
            if bound.trait_ref.bindings.is_empty() {
                continue;
            }
            let subject = bound.subject.substitute(subst);
            // A subject still open at its root has no impl to read an
            // associated type off. One that merely carries an open argument
            // does: `Values<usize, L<T>>` has an `Iterator::Item` of `&L<T>`
            // whatever `T` is.
            if matches!(subject, Ty::Param(_) | Ty::Infer | Ty::Assoc { .. }) {
                continue;
            }
            let asked = TraitRef {
                id: bound.trait_ref.id,
                args: bound.trait_ref.args.iter().map(|a| a.substitute(subst)).collect(),
                bindings: Vec::new(),
            };
            for (name, wanted) in &bound.trait_ref.bindings {
                let Some(actual) = self.project(&subject, Some(&asked), name) else {
                    continue;
                };
                // What the impl supplies can itself be a projection —
                // `Skip<I>`'s `Item` is `<I as Iterator>::Item` — and a
                // projection matches nothing until it is read through.
                let actual = self.normalize(&actual);
                let Some(found) = def.match_written(&wanted.substitute(subst), &actual) else {
                    continue;
                };
                for (param, ty) in found {
                    subst.entry(param).or_insert(ty);
                }
            }
        }
    }

    /// Does this impl supply the associated types the bound names?
    ///
    /// The bound may leave a binding open — `Item = &'a T` where `T` is one of
    /// the *bound's* own parameters — so the two sides are matched rather than
    /// compared, and an impl that supplies nothing for a named binding does not
    /// answer the bound.
    pub(super) fn bindings_agree(
        &self,
        implemented: &TraitRef,
        required: &TraitRef,
        def: &super::impls::ImplDef,
        subst: &Subst,
    ) -> bool {
        let _ = implemented;
        for (name, wanted) in &required.bindings {
            let Some(supplied) = def.assoc_types.get(name) else {
                return false;
            };
            let supplied = self.normalize(&supplied.substitute(subst));
            if supplied == *wanted {
                continue;
            }
            // The wanted side can still hold parameters of whatever wrote the
            // bound — `I: Iterator<Item = &'a T>` names a `T` the impl knows
            // nothing about — and those are holes to be filled, not a licence
            // to accept anything. Unifying fills them and refuses a shape that
            // cannot be made to fit: a wanted `Item = Vec<T>` is not satisfied
            // by an `Item = Option<u8>` however open the `T` is.
            let holes = open_params(wanted);
            if !holes.is_empty()
                && crate::ty::unify(&holes, wanted, &supplied, &mut Subst::new()).is_ok()
            {
                continue;
            }
            return false;
        }
        true
    }
}
