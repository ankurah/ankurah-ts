//! Which function a call lands on, when more than one answers to the name.
//!
//! Rust has two tiers, and so does this: the inherent methods of the type, then
//! every extension candidate. Two answers in one tier is an ambiguity; there is
//! no first-match tie-break. What ranks candidates ACROSS the deref chain is
//! `walk_chain`, which prefers one that applies outright.

use super::{
    AutoRef, Callee, DerefStep, MethodError, MethodResolution, Obligation, Pick, Probe, Ty,
    Undecided,
};

impl Probe<'_> {
    /// The one method that answers at this receiver and borrow, or nothing.
    ///
    /// Rust has two tiers, and so does this: the inherent methods of the type,
    /// then every extension candidate — a trait impl written for a definite
    /// type, an impl written for one of its own parameters, and the declaration
    /// a `dyn Trait` or a bounded parameter dispatches through. Coherence means
    /// one trait cannot have two impls for one type, so splitting the extension
    /// tier further would only ever hide a clash between two *different* traits,
    /// which is exactly the clash Rust reports. Two answers in a tier is an
    /// ambiguity; there is no first-match tie-break.
    pub(super) fn pick(
        &self,
        candidate: &Ty,
        autoref: AutoRef,
        name: &str,
        explicit: &[Ty],
        in_scope_only: bool,
    ) -> Result<Option<Pick>, MethodError> {
        let adjusted = autoref.apply(candidate);

        let inherent = self.impl_picks(candidate, &adjusted, name, true, explicit);
        if let Some(pick) = self.exactly_one(candidate, inherent)? {
            return Ok(Some(pick));
        }

        let mut extension = self.impl_picks(candidate, &adjusted, name, false, explicit);
        // A `dyn Trait` receiver, and a generic parameter bounded by a trait,
        // dispatch through the trait's own declaration. A written
        // `impl Trait for dyn Trait` says the same thing more precisely, so
        // where both are present the impl is the answer rather than a clash.
        //
        // Unless the impl only applies IF something nobody can decide holds.
        // `impl<I: Iterator> IntoIterator for I` matches every receiver and
        // leaves `I: Iterator` deferred; a caller that wrote
        // `fn f<I: IntoIterator>(values: I)` has SAID that `I` implements the
        // trait, and that is the more precise answer, not the blanket resting
        // on a bound the engine cannot close. Written the other way,
        // `values.into_iter()` resolved through the blanket and came out as
        // `values.intoIter()` — a method nothing declares (G1).
        for declared in self.declared_picks(candidate, &adjusted, name, explicit) {
            let same_trait = |p: &Pick| {
                p.callee
                    .impl_id()
                    .and_then(|id| self.reg.impl_def(id).trait_ref.as_ref().map(|t| t.id))
                    == match &declared.callee {
                        Callee::TraitObject(id, ..) => Some(id.clone()),
                        _ => None,
                    }
            };
            let settled = extension.iter().any(|p| same_trait(p) && p.obligations.is_empty());
            if settled {
                continue;
            }
            extension.retain(|p| !same_trait(p));
            extension.push(declared);
        }
        self.exactly_one(candidate, self.nameable(extension, in_scope_only))
    }

    /// The extension candidates whose trait the calling module can name.
    ///
    /// Rust needs the trait in scope for the method to exist at all, and that
    /// is a filter, not a tie-break: the std surface declares reflexive
    /// blankets — `impl<T: ?Sized> BorrowMut<T> for T`, and the same for
    /// `Borrow` and `AsRef` — which answer to `borrow_mut` on *every* receiver
    /// at depth 0. Keeping them made `guard.borrow_mut()` on a
    /// `RwLockReadGuard<RefCell<T>>` resolve to the blanket instead of
    /// `RefCell::borrow_mut` one deref later, and the `.value` accessor the
    /// guard needs was never written.
    ///
    /// The filter runs over the whole deref chain first. Only when nothing in
    /// scope answers anywhere is the unfiltered list allowed to stand — a gap
    /// in the `use` map must not silently delete the only method there is, and
    /// `out_of_scope` on the resolution reports the survivor instead.
    fn nameable(&self, picks: Vec<Pick>, in_scope_only: bool) -> Vec<Pick> {
        let in_scope: Vec<Pick> = picks
            .iter()
            .filter(|p| self.trait_in_scope(&p.callee))
            .cloned()
            .collect();
        if in_scope.is_empty() && !in_scope_only {
            picks
        } else {
            in_scope
        }
    }

    fn exactly_one(&self, candidate: &Ty, picks: Vec<Pick>) -> Result<Option<Pick>, MethodError> {
        // One function reachable by two routes is one answer, not a clash. The
        // same trait method arrives twice wherever a supertrait and a subtrait
        // both offer it, and counting the copies reported `Iterator::find` as
        // ambiguous with itself.
        let mut picks = picks.into_iter().fold(Vec::new(), |mut kept: Vec<Pick>, pick| {
            if !kept.iter().any(|p| p.callee == pick.callee) {
                kept.push(pick);
            }
            kept
        });
        match picks.len() {
            0 => Ok(None),
            1 => Ok(Some(picks.remove(0))),
            _ => Err(MethodError::Ambiguous {
                at: candidate.clone(),
                candidates: picks.into_iter().map(|p| p.callee).collect(),
            }),
        }
    }

    /// The first step of the deref chain that answers to `name`, preferring a
    /// candidate that applies outright.
    ///
    /// `B` not meeting `Red` is a fact, so `w.go()` on a `Wrap<B>` reaches
    /// `Inner::go` through `Deref`. An unknown standing where `B` will be is a
    /// question the engine has not closed, so an unsettled candidate stops the
    /// walk at its own depth and its obligation travels with the answer.
    pub(super) fn walk_chain(
        &self,
        candidates: &[Ty],
        steps: &[DerefStep],
        name: &str,
        explicit: &[Ty],
        undecided: &[Obligation],
        in_scope_only: bool,
    ) -> Result<Option<MethodResolution>, MethodError> {
        let mut deferred: Option<MethodResolution> = None;
        let mut unsettled_at: Option<usize> = None;
        let mut contested = false;
        'chain: for (depth, candidate) in candidates.iter().enumerate() {
            for autoref in [AutoRef::None, AutoRef::Shared, AutoRef::Mut] {
                let found = self.pick(candidate, autoref, name, explicit, in_scope_only)?;
                let Some(pick) = found else { continue };
                // Past an unsettled candidate the walk only counts: a second
                // answer here means which method Rust calls is decided by the
                // bound, so the caller must not read this call's result yet.
                if unsettled_at.is_some_and(|at| depth > at) {
                    contested = true;
                    break 'chain;
                }
                let ret = self.normalize(&pick.ret);
                let outright = pick.obligations.is_empty();
                let unsettled = pick
                    .obligations
                    .iter()
                    .any(|o| o.reason == Undecided::Unsettled);
                let mut obligations = undecided.to_vec();
                obligations.extend(pick.obligations);
                let out_of_scope = (!self.trait_in_scope(&pick.callee))
                    .then(|| self.trait_of(&pick.callee))
                    .flatten();
                let found = MethodResolution {
                    steps: steps[..depth].to_vec(),
                    autoref,
                    callee: pick.callee,
                    subst: pick.subst,
                    ret,
                    adjusted: autoref.apply(candidate),
                    obligations,
                    out_of_scope,
                    contested: false,
                };
                if outright {
                    return Ok(Some(found));
                }
                if unsettled {
                    unsettled_at.get_or_insert(depth);
                }
                deferred.get_or_insert(found);
            }
        }
        if let Some(found) = &mut deferred {
            found.contested = contested;
        }
        Ok(deferred)
    }
}
