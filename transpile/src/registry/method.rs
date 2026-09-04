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

use super::impls::{head_of, Bound, Head, ImplId};
use super::{ModuleId, Ns, TypeRegistry};
use crate::ty::subst::Subst;
use crate::ty::{bind_params, TraitRef, Ty, TypeId};

/// How far a chain of `Deref`s is followed before the engine calls it a cycle.
/// A real chain in the corpus is three steps; sixteen is a bug in the source or
/// in this file, and either way it is a diagnostic rather than a hang.
const MAX_DEREF_STEPS: usize = 16;

/// How deep bound checking recurses looking for an impl. `T: Clone` may need
/// `U: Clone` to hold, but not forever.
const MAX_BOUND_DEPTH: usize = 4;

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
    /// parameter: the trait's own declaration is all there is.
    TraitObject(TypeId, String),
    /// A method on an impl written for one of its own parameters, such as
    /// `impl<T: Display> ToString for T`.
    Blanket(ImplId, String),
    /// The lock-guard shim: the port's `RwLock::read` yields the guard where
    /// Rust yields a `LockResult`, so the `unwrap` written on it has nothing to
    /// do. Named here so the shim is visible in a resolution rather than hidden
    /// in the translator (spec 4.4, deleted when the stubs land).
    GuardShim,
}

impl Callee {
    pub fn impl_id(&self) -> Option<ImplId> {
        match self {
            Callee::Inherent(id, _) | Callee::TraitImpl(id, _) | Callee::Blanket(id, _) => {
                Some(*id)
            }
            Callee::TraitObject(..) | Callee::GuardShim => None,
        }
    }
}

/// A bound the engine recorded rather than decided.
///
/// `impl<F: Fn(T)> IntoBroadcastListener<T> for F` applies only to a closure,
/// and until closures are typed (spec 4.5) the engine cannot say whether a given
/// `F` is one. Assuming it holds would pick an impl that may be wrong; assuming
/// it fails would lose the only impl there is. So the impl stays a candidate and
/// the undecided bound travels with the answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Obligation {
    pub subject: Ty,
    pub bound: TraitRef,
    pub reason: Undecided,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Undecided {
    /// The trait has no declaration in reach: a marker such as `Send`, or a std
    /// trait before the stub declarations land (spec 4.4, step 3).
    NoDeclaration,
    /// The subject is still a type parameter, so there is no type to look for an
    /// impl of.
    OpenSubject,
    /// Deciding it would have recursed past the depth limit.
    DepthLimit,
}

/// Everything a resolved call tells emission.
#[derive(Debug, Clone)]
pub struct MethodResolution {
    /// The dereferences taken from the written receiver, in order.
    pub steps: Vec<DerefStep>,
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


    // ── The deref chain ────────────────────────────────────────────────

    /// One hop: `&T` to `T`, or through the `Deref` impl written for it.
    pub fn deref_once(&self, ty: &Ty) -> Option<DerefStep> {
        self.deref_once_reporting(ty, &mut Vec::new())
    }

    /// The same, collecting the bounds that stopped a conditional `Deref` from
    /// being taken, so the caller can say why the type behind it was not
    /// reached instead of silently stopping the chain.
    fn deref_once_reporting(&self, ty: &Ty, undecided: &mut Vec<Obligation>) -> Option<DerefStep> {
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
            let Some(subst) = def.match_self(ty) else {
                continue;
            };
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
    /// A declared system wrapper says so itself: `Some("value")` is a field,
    /// `Some("")` means the wrapper is transparent and nothing is written.
    /// Anything else went through a crate's own `impl Deref`, which the emitted
    /// class carries as a `deref()` method — Rust inserts that call, and so must
    /// the TypeScript, or the field behind the wrapper is read off the wrapper.
    fn step_accessor(&self, ty: &Ty) -> Option<Accessor> {
        let Some(id) = ty.id() else {
            return Some(Accessor::Call("deref".to_string()));
        };
        if !self.reg.is_system(id) {
            return Some(Accessor::Call("deref".to_string()));
        }
        match self.reg.def(id)?.deref_field.as_deref() {
            None | Some("") => None,
            Some(field) => Some(Accessor::Field(field.to_string())),
        }
    }

    /// Every receiver reachable from the written one, in the order Rust tries
    /// them: itself, then each dereference, then the unsized form of the last.
    pub fn deref_chain(&self, receiver: &Ty) -> Result<Vec<DerefStep>, MethodError> {
        self.deref_chain_reporting(receiver, &mut Vec::new())
    }

    fn deref_chain_reporting(
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

    // ── Method resolution ──────────────────────────────────────────────

    /// Which function `receiver.name(..)` calls.
    pub fn resolve_method(&self, receiver: &Ty, name: &str) -> Result<MethodResolution, MethodError> {
        let mut undecided = Vec::new();
        let steps = self.deref_chain_reporting(receiver, &mut undecided)?;
        let mut candidates: Vec<Ty> = vec![receiver.clone()];
        candidates.extend(steps.iter().map(|s| s.to.clone()));

        for (depth, candidate) in candidates.iter().enumerate() {
            for autoref in [AutoRef::None, AutoRef::Shared, AutoRef::Mut] {
                let found = self.pick(candidate, autoref, name)?;
                let Some(pick) = found else { continue };
                let ret = self.normalize(&pick.ret);
                let mut obligations = undecided;
                obligations.extend(pick.obligations);
                let out_of_scope = (!self.trait_in_scope(&pick.callee))
                    .then(|| self.trait_of(&pick.callee))
                    .flatten();
                return Ok(MethodResolution {
                    steps: steps[..depth].to_vec(),
                    autoref,
                    callee: pick.callee,
                    subst: pick.subst,
                    ret,
                    adjusted: autoref.apply(candidate),
                    obligations,
                    out_of_scope,
                });
            }
        }

        Err(MethodError::NotFound {
            receiver: receiver.clone(),
            tried: candidates,
        })
    }

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
    fn pick(&self, candidate: &Ty, autoref: AutoRef, name: &str) -> Result<Option<Pick>, MethodError> {
        let adjusted = autoref.apply(candidate);

        let inherent = self.impl_picks(candidate, &adjusted, name, true);
        if let Some(pick) = self.exactly_one(candidate, inherent)? {
            return Ok(Some(pick));
        }

        let mut extension = self.impl_picks(candidate, &adjusted, name, false);
        // A `dyn Trait` receiver, and a generic parameter bounded by a trait,
        // dispatch through the trait's own declaration. A written
        // `impl Trait for dyn Trait` says the same thing more precisely, so
        // where both are present the impl is the answer rather than a clash.
        for declared in self.declared_picks(candidate, &adjusted, name) {
            let already = extension.iter().any(|p| {
                p.callee
                    .impl_id()
                    .and_then(|id| self.reg.impl_def(id).trait_ref.as_ref().map(|t| t.id))
                    == match &declared.callee {
                        Callee::TraitObject(id, _) => Some(*id),
                        _ => None,
                    }
            });
            if !already {
                extension.push(declared);
            }
        }
        self.exactly_one(candidate, extension)
    }

    fn exactly_one(&self, candidate: &Ty, mut picks: Vec<Pick>) -> Result<Option<Pick>, MethodError> {
        match picks.len() {
            0 => Ok(None),
            1 => Ok(Some(picks.remove(0))),
            _ => {
                // Rust settles this with the trait's visibility: a trait whose
                // name the module cannot see contributes no method. The engine
                // applies that only here, where it changes the answer, so that a
                // gap in the import map cannot silently lose a method that has
                // no competition.
                let in_scope: Vec<Pick> = picks
                    .iter()
                    .filter(|p| self.trait_in_scope(&p.callee))
                    .cloned()
                    .collect();
                if in_scope.len() == 1 {
                    return Ok(Some(in_scope.into_iter().next().unwrap()));
                }
                Err(MethodError::Ambiguous {
                    at: candidate.clone(),
                    candidates: picks.into_iter().map(|p| p.callee).collect(),
                })
            }
        }
    }

    /// The trait a callee came through, when it came through one.
    fn trait_of(&self, callee: &Callee) -> Option<TypeId> {
        match callee {
            Callee::Inherent(..) | Callee::GuardShim => None,
            Callee::TraitObject(id, _) => Some(*id),
            Callee::TraitImpl(id, _) | Callee::Blanket(id, _) => {
                self.reg.impl_def(*id).trait_ref.as_ref().map(|tr| tr.id)
            }
        }
    }

    /// Is the trait this callee came from nameable from the module that wrote
    /// the call?
    ///
    /// Rust needs the trait in scope for the method to exist at all. The engine
    /// only *reports* a sole candidate whose trait it cannot name, rather than
    /// deleting the method: the answer would then depend on the `use` map being
    /// complete, and a gap there would silently remove a method instead of
    /// showing up in the diagnostics. Where two candidates compete it does
    /// decide, because there the answer turns on it.
    fn trait_in_scope(&self, callee: &Callee) -> bool {
        let Some(trait_id) = self.trait_of(callee) else {
            return true;
        };
        let name = self.reg.name_of(trait_id);
        matches!(
            self.reg.lookup(self.module, Ns::Type, &[name]),
            Ok(Some(super::Def::Type(found))) if found == trait_id
        )
    }

    /// Methods declared by a trait the candidate is known to implement because
    /// it *is* that trait: `dyn Trait`, or a parameter carrying the bound.
    fn declared_picks(&self, candidate: &Ty, adjusted: &Ty, name: &str) -> Vec<Pick> {
        let bounds: Vec<TraitRef> = match candidate {
            Ty::Dyn { traits } | Ty::ImplTrait { bounds: traits } => traits.clone(),
            Ty::Param(param) => self
                .param_bounds
                .iter()
                .filter(|(p, _)| p == param)
                .map(|(_, t)| t.clone())
                .collect(),
            _ => return Vec::new(),
        };

        let mut picks = Vec::new();
        for bound in &bounds {
            let Some((owner, method)) = self.reg.trait_method(bound.id, name) else {
                continue;
            };
            // The trait's own declaration writes its receiver in terms of
            // `Self`, which here is the object or the bounded parameter.
            let Some(receiver) = &method.sig.receiver else {
                continue;
            };
            let mut self_subst = Subst::new();
            self_subst.insert("Self".to_string(), candidate.clone());
            if &receiver.substitute(&self_subst) != adjusted {
                continue;
            }
            let Some(trait_def) = self.reg.trait_def(bound.id) else {
                continue;
            };
            let mut subst = bind_params(&trait_def.generics, &bound.args);
            subst.insert("Self".to_string(), candidate.clone());
            for (assoc, ty) in &bound.bindings {
                subst.insert(assoc.clone(), ty.clone());
            }
            picks.push(Pick {
                callee: Callee::TraitObject(owner, name.to_string()),
                ret: method.sig.ret.substitute(&subst),
                subst,
                obligations: Vec::new(),
            });
        }
        picks
    }

    fn impl_picks(&self, candidate: &Ty, adjusted: &Ty, name: &str, inherent: bool) -> Vec<Pick> {
        let mut ids: Vec<ImplId> = Vec::new();
        for head in candidate_heads(candidate) {
            for id in self.reg.impls().for_head(&head) {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
        if !inherent {
            for &id in self.reg.impls().blanket() {
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
            let Some(subst) = def.match_written(&receiver, adjusted) else {
                continue;
            };
            let Some((sig, _owner)) = self.impl_method(def, name, &subst) else {
                continue;
            };
            let Some(obligations) = self.bounds_hold(&def.bounds, &subst) else {
                continue;
            };
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
    /// own parameters. The impl's own signature already says `Self`; a default
    /// the trait wrote says it in the trait's terms, so `Self` is filled in.
    fn method_receiver(&self, def: &super::impls::ImplDef, name: &str) -> Option<Ty> {
        if let Some(sig) = def.methods.get(name) {
            return sig.receiver.clone();
        }
        let tr = def.trait_ref.as_ref()?;
        let (_, method) = self.reg.trait_method(tr.id, name)?;
        if !method.has_default {
            return None;
        }
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
        let (owner, method) = self.reg.trait_method(tr.id, name)?;
        if !method.has_default {
            return None;
        }
        // The default body's signature speaks in the trait's parameters and in
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
        };
        Some((sig, Some(owner)))
    }

    // ── Bounds ─────────────────────────────────────────────────────────

    /// Do this impl's `where` clauses hold for the types just bound? `None` when
    /// one of them definitely does not; otherwise the ones the engine could not
    /// decide, travelling with the answer.
    fn bounds_hold(&self, bounds: &[Bound], subst: &Subst) -> Option<Vec<Obligation>> {
        let mut deferred = Vec::new();
        for bound in bounds {
            let subject = bound.subject.substitute(subst);
            let trait_ref = bound.trait_ref.substitute(subst);
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

    fn holds(&self, subject: &Ty, trait_ref: &TraitRef, depth: usize) -> Holds {
        if depth >= MAX_BOUND_DEPTH {
            return Holds::Undecided(Undecided::DepthLimit);
        }
        // A trait nothing declares says nothing about the subject. `Send` and
        // `Fn(T)` are the common ones; the std-surface step declares the second
        // kind and the closure step decides them.
        if self.reg.trait_def(trait_ref.id).is_none() {
            return Holds::Undecided(Undecided::NoDeclaration);
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
        // A parameter that is still open is not a type an impl can be found for.
        if matches!(subject, Ty::Param(_) | Ty::Infer) || subject.has_open_param() {
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
            let Some(subst) = def.match_self(subject) else {
                continue;
            };
            // The trait's own arguments have to agree too: `impl Marker<u16> for
            // S` says nothing about `S: Marker<u8>`.
            let Some(implemented) = def.trait_ref.as_ref() else {
                continue;
            };
            if &implemented.substitute(&subst) != trait_ref {
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
            None => Holds::No,
        }
    }

    // ── Associated types ───────────────────────────────────────────────

    /// Replace every projection the impl table can answer. `<Vec<u8> as
    /// TryInto<Clock>>::Error` becomes whatever that impl wrote for `Error`;
    /// a projection no impl supplies is left standing, which is the truth
    /// about it.
    pub fn normalize(&self, ty: &Ty) -> Ty {
        self.normalize_within(ty, 0)
    }

    fn normalize_within(&self, ty: &Ty, depth: usize) -> Ty {
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
    fn declaring_bound(&self, base: &Ty, name: &str) -> Option<TraitRef> {
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
    fn project(&self, base: &Ty, trait_: Option<&TraitRef>, name: &str) -> Option<Ty> {
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
            let Some(subst) = def.match_self(base) else {
                continue;
            };
            if let Some(tr) = trait_ {
                // The projection names the trait *with its arguments*:
                // `<S as Carrier<u8>>::Item` is not what `impl Carrier<u16> for S`
                // supplies.
                let Some(impl_trait) = def.trait_ref.as_ref() else {
                    continue;
                };
                if &impl_trait.substitute(&subst) != tr {
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
struct Pick {
    callee: Callee,
    ret: Ty,
    subst: Subst,
    obligations: Vec<Obligation>,
}

enum Holds {
    Yes,
    No,
    Undecided(Undecided),
}

impl Ty {
    /// Does any type parameter survive inside this type? A bound on such a type
    /// cannot be looked up, because there is no type yet to look it up for.
    pub fn has_open_param(&self) -> bool {
        match self {
            Ty::Param(_) | Ty::Infer => true,
            Ty::Named { args, .. } | Ty::Tuple(args) => args.iter().any(|a| a.has_open_param()),
            Ty::Ref { inner, .. } | Ty::Slice(inner) | Ty::Array { elem: inner, .. } => {
                inner.has_open_param()
            }
            Ty::Assoc { .. } => true,
            Ty::Dyn { .. } | Ty::ImplTrait { .. } => false,
            Ty::Prim(_) | Ty::Str | Ty::Unit | Ty::Never => false,
        }
    }
}

impl TypeRegistry {
    /// The trait every deref step goes through. Declared with the other system
    /// types; the std-surface step replaces the declaration with a stub and this
    /// keeps working.
    pub fn deref_trait(&self) -> Option<TypeId> {
        self.system_type("std::ops::Deref")
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
    /// The type of `expr.field`, walking the same chain method calls walk.
    pub fn resolve_field(&self, receiver: &Ty, field: &str) -> Option<FieldResolution> {
        let steps = self.deref_chain(receiver).ok()?;
        let mut candidates: Vec<Ty> = vec![receiver.clone()];
        candidates.extend(steps.iter().map(|s| s.to.clone()));
        for (depth, candidate) in candidates.iter().enumerate() {
            if let Some(ty) = self.field_on(candidate, field) {
                return Some(FieldResolution {
                    ty: self.normalize(&ty),
                    steps: steps[..depth].to_vec(),
                });
            }
        }
        None
    }

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

    fn field_on(&self, ty: &Ty, field: &str) -> Option<Ty> {
        let Ty::Named { id, args } = ty else {
            return None;
        };
        let def = self.reg.def(*id)?;
        let subst = bind_params(&def.type_params, args);
        def.fields
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, ty)| ty.substitute(&subst))
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

