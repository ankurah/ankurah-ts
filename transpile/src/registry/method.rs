//! Which function a method call lands on.
//!
//! Rust does not write the answer down. `self.0.listeners.read().unwrap().len()`
//! calls `HashMap::len` through a lock guard, and `self.0.iter()` calls
//! `<[EventId]>::iter` through a `Vec`; both are spelled the same way, and the
//! emitted TypeScript differs. This walks Rust's own algorithm over the impl
//! table: build the receivers reachable by dereferencing, try each borrow at
//! each of them, and take the one step that has exactly one answer.
//!
//! Nothing here guesses. A step with two answers and a chain with none are both
//! reported, naming what was tried.

/// The method a BOUND declares, as a candidate in its own right.
pub(crate) mod declared;
pub(crate) mod ranking;
#[cfg(test)]
mod ranking_tests;
pub(crate) mod deref;
pub(crate) mod signatures;

pub use super::bounds::{Obligation, Undecided};
use super::impls::{head_of, Bound, Head, ImplId};
use super::{ModuleId, TypeRegistry};
use crate::ty::subst::Subst;
use crate::ty::{bind_params, TraitRef, Ty, TypeId};

/// The marker rustc decides from a type's layout rather than from any impl.
pub(super) const SIZED_PATH: &str = "std::marker::Sized";

/// How far a chain of `Deref`s is followed before the engine calls it a cycle.
/// A real chain in the corpus is three steps; sixteen is a bug in the source or
/// in this file, and either way it is a diagnostic rather than a hang.
const MAX_DEREF_STEPS: usize = 16;

/// How deep bound checking recurses looking for an impl. `T: Clone` may need
/// `U: Clone` to hold, but not forever.
pub(super) const MAX_BOUND_DEPTH: usize = 4;

/// One hop from a receiver to what it dereferences to.
#[derive(Debug, Clone, PartialEq)]
pub struct DerefStep {
    pub from: Ty,
    pub to: Ty,
    pub kind: DerefKind,
    /// What TypeScript writes to reach through this hop. `None` for a hop that
    /// is invisible in the emitted code: a `&`, a transparent wrapper such as
    /// `Box`, or an unsizing.
    pub accessor: Option<Accessor>,
}

/// How a dereference is written in TypeScript.
///
/// A declared system wrapper holds its value in a field, so reaching through it
/// is a field read. A crate's own `impl Deref` is a function, and the emitted
/// class carries it as a method, so reaching through that one is a call — the
/// same call Rust inserts.
#[derive(Debug, Clone, PartialEq)]
pub enum Accessor {
    Field(String),
    Call(String),
}

impl Accessor {
    /// The text written after the dot.
    pub fn written(&self) -> String {
        match self {
            Accessor::Field(name) => name.clone(),
            Accessor::Call(name) => format!("{}()", name),
        }
    }

    /// The field this reaches through, if it is one. `*x = y` has to name a
    /// place to assign to, and a call is not one.
    pub fn field(&self) -> Option<&str> {
        match self {
            Accessor::Field(name) => Some(name),
            Accessor::Call(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DerefKind {
    /// `&T` to `T`, which the language does without an impl.
    Builtin,
    /// Through an `impl Deref`.
    Overloaded(ImplId),
    /// `[T; N]` to `[T]`.
    Unsize,
}

/// The borrow taken of the receiver before the call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoRef {
    None,
    Shared,
    Mut,
}

impl AutoRef {
    fn apply(self, ty: &Ty) -> Ty {
        match self {
            AutoRef::None => ty.clone(),
            AutoRef::Shared => Ty::Ref {
                mutable: false,
                inner: Box::new(ty.clone()),
            },
            AutoRef::Mut => Ty::Ref {
                mutable: true,
                inner: Box::new(ty.clone()),
            },
        }
    }
}

/// The function a call resolved to.
#[derive(Debug, Clone, PartialEq)]
pub enum Callee {
    /// A method on an `impl Type { .. }` block.
    Inherent(ImplId, String),
    /// A method on an `impl Trait for Type { .. }` block written for a definite
    /// type.
    TraitImpl(ImplId, String),
    /// A method reached through `dyn Trait`, or through a bound on a generic
    /// parameter: the trait's own declaration is all there is. H13: the witness
    /// is what makes `declared_picks` the only producer of one.
    TraitObject(TypeId, String, declared::DeclaredBound),
    /// A method on an impl written for one of its own parameters, such as
    /// `impl<T: Display> ToString for T`.
    Blanket(ImplId, String),
}

impl Callee {
    pub fn impl_id(&self) -> Option<ImplId> {
        match self {
            Callee::Inherent(id, _) | Callee::TraitImpl(id, _) | Callee::Blanket(id, _) => {
                Some(*id)
            }
            Callee::TraitObject(..) => None,
        }
    }
}

/// Everything a resolved call tells emission.
#[derive(Debug, Clone)]
pub struct MethodResolution {
    /// The dereferences taken from the written receiver, in order.
    pub steps: Vec<DerefStep>,
    /// The borrow taken of the receiver before the call. Emission erases it —
    /// a JavaScript method call takes no borrow — and the tests assert it,
    /// because it is what says which step of Rust's probe was taken.
    /// Which auto-reference the probe took to reach the method. Read by the
    /// resolution's own tests, which pin the step Rust's probe takes; emission
    /// writes the receiver from the accessors rather than from this.
    #[cfg_attr(not(test), allow(dead_code))]
    pub autoref: AutoRef,
    pub callee: Callee,
    /// The impl's parameters, bound to what stood at their positions.
    pub subst: Subst,
    /// The call's type.
    pub ret: Ty,
    /// The receiver type the callee sees, after the steps and the borrow.
    pub adjusted: Ty,
    /// Bounds the engine recorded but could not decide. Step 4 discharges
    /// these; until then each one is reported where the call is translated, so
    /// a resolution that rests on an open question is visible.
    pub obligations: Vec<Obligation>,
    /// The trait the method came through, when the module that wrote the call
    /// cannot name it. Rust would not admit the method at all; the engine takes
    /// it and says so.
    pub out_of_scope: Option<TypeId>,
}

impl MethodResolution {
    /// The accessors TypeScript has to write between the receiver and the call,
    /// in order. A step through a transparent wrapper contributes nothing.
    pub fn accessors(&self) -> Vec<String> {
        self.steps
            .iter()
            .filter_map(|s| s.accessor.as_ref().map(|a| a.written()))
            .collect()
    }

    /// The type the callee is written for, with the borrow stripped: what the
    /// native-type translations dispatch on.
    pub fn receiver_type(&self) -> &Ty {
        self.adjusted.peel_refs()
    }
}

/// Why a call could not be resolved to one function.
#[derive(Debug, Clone)]
pub enum MethodError {
    /// Nothing in the impl table answers to that name anywhere along the chain.
    NotFound { receiver: Ty, tried: Vec<Ty> },
    /// Two impls answer at the same step, and Rust would need a qualified path
    /// to say which.
    Ambiguous { at: Ty, candidates: Vec<Callee> },
    /// A `Deref` chain that does not end.
    DerefCycle { receiver: Ty },
}

/// What a call is being resolved in: the module that wrote it, and the bounds
/// on the type parameters in scope, so that `self.notify()` inside a trait's
/// own default body reaches the trait's declaration.
pub struct Probe<'a> {
    pub reg: &'a TypeRegistry,
    pub module: ModuleId,
    pub param_bounds: &'a [(String, TraitRef)],
}

impl<'a> Probe<'a> {
    pub fn new(reg: &'a TypeRegistry, module: ModuleId) -> Probe<'a> {
        Probe {
            reg,
            module,
            param_bounds: &[],
        }
    }

    pub fn with_bounds(mut self, bounds: &'a [(String, TraitRef)]) -> Probe<'a> {
        self.param_bounds = bounds;
        self
    }

    // ── Method resolution ──────────────────────────────────────────────

    /// Which function `receiver.name(..)` calls.
    ///
    /// Only the tests reach it: translation always has a turbofish to pass,
    /// even when it is empty, and goes through `resolve_method_with`.
    #[cfg(test)]
    pub fn resolve_method(&self, receiver: &Ty, name: &str) -> Result<MethodResolution, MethodError> {
        self.resolve_method_with(receiver, name, &[])
    }

    /// The same, with the type arguments a turbofish wrote.
    ///
    /// `collect::<Vec<_>>()` is the reason this exists: `Iterator::collect`
    /// returns its own parameter `B`, so nothing about the receiver says what
    /// the call produces and only the turbofish does.
    pub fn resolve_method_with(
        &self,
        receiver: &Ty,
        name: &str,
        explicit: &[Ty],
    ) -> Result<MethodResolution, MethodError> {
        let mut undecided = Vec::new();
        let steps = self.deref_chain_reporting(receiver, &mut undecided)?;
        let mut candidates: Vec<Ty> = vec![receiver.clone()];
        candidates.extend(steps.iter().map(|s| s.to.clone()));

        // Rust needs the trait in scope for the method to exist at all, so the
        // whole deref chain is walked once with that filter on. Applying it a
        // step at a time re-admitted the reflexive blankets — `impl<T: ?Sized>
        // BorrowMut<T> for T` answers `borrow_mut` on every receiver at depth 0
        // — and a `RwLockReadGuard<RefCell<T>>` then resolved to the blanket
        // instead of `RefCell::borrow_mut` one deref later.
        if let Some(found) =
            self.walk_chain(&candidates, &steps, name, explicit, &undecided, true)?
        {
            return Ok(found);
        }
        // Nothing in scope answers anywhere along the chain. A gap in the `use`
        // map must not silently delete the only method there is, so the
        // unfiltered answer stands and `out_of_scope` reports it.
        if let Some(found) =
            self.walk_chain(&candidates, &steps, name, explicit, &undecided, false)?
        {
            return Ok(found);
        }

        Err(MethodError::NotFound {
            receiver: receiver.clone(),
            tried: candidates,
        })
    }

    /// The first step of the deref chain that answers to `name`, preferring a
    /// candidate that applies outright.
    ///
    /// A candidate that applies only IF something nobody has decided holds
    /// ranks below one that applies outright, at EVERY depth: `w.go()` on a
    /// `Wrap<B>` reaches `Inner::go` through `Deref`, where the extension impl
    /// written `impl<T: Red> Ext for Wrap<T>` wants a bound `B` does not meet.
    fn walk_chain(
        &self,
        candidates: &[Ty],
        steps: &[DerefStep],
        name: &str,
        explicit: &[Ty],
        undecided: &[Obligation],
        in_scope_only: bool,
    ) -> Result<Option<MethodResolution>, MethodError> {
        let mut deferred: Option<MethodResolution> = None;
        for (depth, candidate) in candidates.iter().enumerate() {
            for autoref in [AutoRef::None, AutoRef::Shared, AutoRef::Mut] {
                let found = self.pick(candidate, autoref, name, explicit, in_scope_only)?;
                let Some(pick) = found else { continue };
                let ret = self.normalize(&pick.ret);
                let outright = pick.obligations.is_empty();
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
                };
                if outright {
                    return Ok(Some(found));
                }
                deferred.get_or_insert(found);
            }
        }
        Ok(deferred)
    }

    /// The trait a callee came through, when it came through one.
    pub(super) fn trait_of(&self, callee: &Callee) -> Option<TypeId> {
        match callee {
            Callee::Inherent(..) => None,
            Callee::TraitObject(id, ..) => Some(*id),
            Callee::TraitImpl(id, _) | Callee::Blanket(id, _) => {
                self.reg.impl_def(*id).trait_ref.as_ref().map(|tr| tr.id)
            }
        }
    }


    fn impl_picks(
        &self,
        candidate: &Ty,
        adjusted: &Ty,
        name: &str,
        inherent: bool,
        explicit: &[Ty],
    ) -> Vec<Pick> {
        let mut ids: Vec<ImplId> = Vec::new();
        for head in candidate_heads(candidate) {
            for id in self.reg.impls().for_head(&head) {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
        if !inherent {
            for &id in self.reg.impls().blanket_offering(name) {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }

        let mut picks = Vec::new();
        for id in ids {
            let def = self.reg.impl_def(id);
            if def.is_inherent() != inherent {
                continue;
            }
            let Some(receiver) = self.method_receiver(def, name) else {
                continue;
            };
            let Some(mut subst) = def.match_written(&receiver, adjusted) else {
                continue;
            };
            self.infer_from_bounds(def, &mut subst);
            let Some((sig, _owner)) = self.impl_method(def, name, &subst) else {
                continue;
            };
            let Some(obligations) = self.bounds_hold(&def.bounds, &subst) else {
                continue;
            };
            let mut subst = subst;
            self.bind_explicit(&sig, explicit, &mut subst);
            // A method's own `where` clause is as binding as its impl's.
            // `fn entry(&mut self, key: K) -> Entry<..> where K: Eq + Hash`
            // rules the method out for a key that implements neither, and
            // storing the clause without evaluating it let every such method
            // answer for every receiver.
            let decidable: Vec<Bound> = sig
                .bounds
                .iter()
                .filter(|b| !still_open(b, &subst))
                .cloned()
                .collect();
            let Some(method_obligations) = self.bounds_hold(&decidable, &subst) else {
                continue;
            };
            let mut obligations = obligations;
            obligations.extend(method_obligations);
            let ret = sig.ret.substitute(&subst);
            let callee = if inherent {
                Callee::Inherent(id, name.to_string())
            } else if def.is_blanket() {
                Callee::Blanket(id, name.to_string())
            } else {
                Callee::TraitImpl(id, name.to_string())
            };
            picks.push(Pick {
                callee,
                ret,
                subst,
                obligations,
            });
        }
        picks
    }

    /// The receiver type a method of this impl accepts, written in the impl's
    /// own parameters. The impl's own signature already says `Self`; one the
    /// trait declared says it in the trait's terms, so `Self` is filled in.
    ///
    /// An impl supplies every method its trait declares, whether or not the
    /// trait wrote a body: the corpus compiles, so an impl that writes none of
    /// them has inherited every one. Requiring a written default here made
    /// `values().cloned()` unresolvable, because `Iterator` declares `cloned`
    /// and `impl Iterator for Values` writes only `next`.
    fn method_receiver(&self, def: &super::impls::ImplDef, name: &str) -> Option<Ty> {
        if let Some(sig) = def.methods.get(name) {
            return sig.receiver.clone();
        }
        let tr = def.trait_ref.as_ref()?;
        let method = self.reg.trait_own_method(tr.id, name)?;
        let mut self_subst = Subst::new();
        self_subst.insert("Self".to_string(), def.self_ty.clone());
        method.sig.receiver.as_ref().map(|r| r.substitute(&self_subst))
    }

    /// The signature an impl supplies for a method: the one it writes, or the
    /// default the trait wrote. The returned `subst` additions bind the trait's
    /// own parameters, which a default body's signature is written in.
    fn impl_method(
        &self,
        def: &super::impls::ImplDef,
        name: &str,
        subst: &Subst,
    ) -> Option<(super::MethodSig, Option<TypeId>)> {
        if let Some(sig) = def.methods.get(name) {
            return Some((sig.clone(), None));
        }
        let tr = def.trait_ref.as_ref()?;
        let method = self.reg.trait_own_method(tr.id, name)?;
        let owner = tr.id;
        // The inherited signature speaks in the trait's parameters and in
        // `Self`; both are known here.
        let trait_def = self.reg.trait_def(tr.id)?;
        let mut trait_subst = bind_params(&trait_def.generics, &tr.args);
        trait_subst.insert("Self".to_string(), def.self_ty.clone());
        for (assoc, ty) in &tr.bindings {
            trait_subst.insert(assoc.clone(), ty.clone());
        }
        let sig = super::MethodSig {
            params: method
                .sig
                .params
                .iter()
                .map(|(n, t)| (n.clone(), t.substitute(&trait_subst).substitute(subst)))
                .collect(),
            ret: method.sig.ret.substitute(&trait_subst),
            self_kind: method.sig.self_kind,
            receiver: method.sig.receiver.as_ref().map(|r| r.substitute(&trait_subst)),
            type_params: method.sig.type_params.clone(),
            is_async: method.sig.is_async,
            bounds: method
                .sig
                .bounds
                .iter()
                .map(|b| super::impls::Bound {
                    subject: b.subject.substitute(&trait_subst),
                    trait_ref: b.trait_ref.substitute(&trait_subst),
                })
                .collect(),
        };
        Some((sig, Some(owner)))
    }

    /// Bind the method's own parameters to the types a turbofish wrote.
    ///
    /// A turbofish may leave holes — `collect::<Vec<_>>()` says the shape and
    /// not the element — and the method's own bound is what fills them:
    /// `B: FromIterator<Self::Item>` with `B = Vec<_>` picks
    /// `impl<T> FromIterator<T> for Vec<T>` and reads `T` off the item type. A
    /// hole no bound can fill stays `Infer`, which is the truth about it.
    pub(super) fn bind_explicit(&self, sig: &super::MethodSig, explicit: &[Ty], subst: &mut Subst) {
        for (param, written) in sig.type_params.iter().zip(explicit) {
            let written = self.normalize(&written.substitute(subst));
            let filled = if written.contains_infer() {
                self.fill_holes(param, &written, sig, subst)
            } else {
                written
            };
            subst.insert(param.clone(), filled);
        }
    }

    /// The type a partly-written turbofish argument stands for, found by asking
    /// the impl table which impl of the bound's trait has that shape.
    fn fill_holes(&self, param: &str, written: &Ty, sig: &super::MethodSig, subst: &Subst) -> Ty {
        let head = head_of(written, &[]);
        for bound in &sig.bounds {
            if bound.subject != Ty::Param(param.to_string()) {
                continue;
            }
            let want = self.normalize_trait_ref(&bound.trait_ref.substitute(subst));
            for &id in self.reg.impls().of_trait(want.id) {
                let def = self.reg.impl_def(id);
                if head_of(&def.self_ty, &def.generics) != head {
                    continue;
                }
                let Some(implemented) = def.trait_ref.as_ref() else {
                    continue;
                };
                // The impl's own parameters are the unknowns: matching
                // `FromIterator<T>` against `FromIterator<Listener>` binds `T`,
                // and `Vec<T>` with that binding is the answer.
                let Some(bound_subst) = def.match_written_args(implemented, &want) else {
                    continue;
                };
                let candidate = def.self_ty.substitute(&bound_subst);
                if !candidate.contains_infer() {
                    return candidate;
                }
            }
        }
        written.clone()
    }

    pub(super) fn normalize_trait_ref(&self, tr: &TraitRef) -> TraitRef {
        TraitRef {
            id: tr.id,
            args: tr.args.iter().map(|a| self.normalize(a)).collect(),
            bindings: tr
                .bindings
                .iter()
                .map(|(n, t)| (n.clone(), self.normalize(t)))
                .collect(),
        }
    }

}

/// The self-type shapes whose impls could accept a receiver of this type.
///
/// A method's receiver is its impl's self type with a borrow in front, and the
/// adjusted receiver is the candidate with a borrow in front, so an impl can be
/// reached either with the borrows lining up — the candidate's own shape — or
/// through the referent when the candidate is already a reference and the
/// method takes one. An impl written for a reference type is filed under `Ref`
/// and is reachable from both.
fn candidate_heads(candidate: &Ty) -> Vec<Head> {
    let mut heads = vec![head_of(candidate, &[]), Head::Ref];
    if let Ty::Ref { inner, .. } = candidate {
        heads.push(head_of(inner, &[]));
    }
    heads.dedup();
    heads
}

#[derive(Debug, Clone)]
pub(super) struct Pick {
    pub(super) callee: Callee,
    pub(super) ret: Ty,
    pub(super) subst: Subst,
    pub(super) obligations: Vec<Obligation>,
}

pub(super) enum Holds {
    Yes,
    No,
    Undecided(Undecided),
}


impl TypeRegistry {
    /// The trait every deref step goes through. Declared with the other system
    /// types; the std-surface step replaces the declaration with a stub and this
    /// keeps working.
    pub fn deref_trait(&self) -> Option<TypeId> {
        self.system_type(super::DEREF_PATH)
    }
}

// ── Fields ─────────────────────────────────────────────────────────────

/// Where a field was found, and what has to be written to reach it.
#[derive(Debug, Clone)]
pub struct FieldResolution {
    pub ty: Ty,
    pub steps: Vec<DerefStep>,
}

impl FieldResolution {
    pub fn accessors(&self) -> Vec<String> {
        self.steps
            .iter()
            .filter_map(|s| s.accessor.as_ref().map(|a| a.written()))
            .collect()
    }
}

impl<'a> Probe<'a> {

    /// Does an impl written for this exact type declare a method of that name,
    /// without dereferencing to find it? The unresolved-call path in the
    /// translator asks this to decide whether to reach through a wrapper.
    pub fn declares_method(&self, ty: &Ty, name: &str) -> bool {
        let head = head_of(ty, &[]);
        self.reg.impls().for_head(&head).any(|id| {
            let def = self.reg.impl_def(id);
            def.methods.contains_key(name) && def.match_self(ty).is_some()
        })
    }

}

/// Bounds written on a trait declaration or an impl block, keyed by the
/// parameter they constrain. Used to seed `Probe::param_bounds`.
pub type ParamBounds = Vec<(String, TraitRef)>;

/// The bounds an impl or a function declares, as parameter/trait pairs.
pub fn param_bounds_of(bounds: &[Bound]) -> ParamBounds {
    bounds
        .iter()
        .filter_map(|b| match &b.subject {
            Ty::Param(name) => Some((name.clone(), b.trait_ref.clone())),
            _ => None,
        })
        .collect()
}

/// Named lookups the registry answers on behalf of the probe.
impl TypeRegistry {
    /// Impls written for a type, whatever trait they implement. Used by the
    /// tests and by the inventory.
    pub fn impls_for(&self, ty: &Ty) -> Vec<ImplId> {
        self.impls().for_head(&head_of(ty, &[])).collect()
    }

    #[allow(dead_code)]
    pub(crate) fn impl_count(&self) -> usize {
        self.impls().len()
    }
}

/// Does this bound still name a type nothing has filled in?
///
/// A method's own `where` clause is as binding as its impl's, but only once
/// there is something to check it against. `fn collect<B: FromIterator<Item>>`
/// written without a turbofish leaves `B` open, and
/// `fn get<Q>(&self, k: &Q) where K: Borrow<Q>` leaves the `Q` inside the bound
/// open even though its subject `K` is fixed. Rust decides both from the
/// argument and the expected type, which is a later step; until then neither
/// says whether the method applies, and treating them as failures deleted
/// `HashMap::get` and `Iterator::collect` outright. A bound the substitution
/// closed — `K: Eq + Hash` on a `HashMap<EntityId, u8>` — is decided normally.
pub(super) fn still_open(bound: &Bound, subst: &Subst) -> bool {
    if !open_params(&bound.subject.substitute(subst)).is_empty() {
        return true;
    }
    bound
        .trait_ref
        .args
        .iter()
        .any(|arg| !open_params(&arg.substitute(subst)).is_empty())
        || bound
            .trait_ref
            .bindings
            .iter()
            .any(|(_, b)| !open_params(&b.substitute(subst)).is_empty())
}

/// The parameter names a written type still leaves open, which are the holes a
/// bound's associated binding has to be matched through.
pub(super) fn open_params(ty: &Ty) -> Vec<String> {
    let mut names = Vec::new();
    collect_params(ty, &mut names);
    names.sort();
    names.dedup();
    names
}

pub(super) fn collect_params(ty: &Ty, out: &mut Vec<String>) {
    match ty {
        Ty::Param(name) => out.push(name.clone()),
        Ty::Named { args, .. } | Ty::Tuple(args) => args.iter().for_each(|a| collect_params(a, out)),
        Ty::Ref { inner, .. } | Ty::Slice(inner) | Ty::Array { elem: inner, .. } => {
            collect_params(inner, out)
        }
        Ty::Assoc { base, .. } => collect_params(base, out),
        Ty::Dyn { traits } | Ty::ImplTrait { bounds: traits } => {
            for t in traits {
                t.args.iter().for_each(|a| collect_params(a, out));
                t.bindings.iter().for_each(|(_, b)| collect_params(b, out));
            }
        }
        Ty::Prim(_) | Ty::Str | Ty::Unit | Ty::Never | Ty::Infer | Ty::Var(_) => {}
    }
}
