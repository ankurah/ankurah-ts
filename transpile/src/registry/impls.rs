//! Every `impl` block, indexed by what it is written for.
//!
//! Merging an impl's methods into the type it names loses the two facts method
//! resolution runs on: which trait the method came from, and what shape the impl
//! is actually written for. `impl<T> Signal for Arc<Inner<T>>` is an impl on
//! `Arc<Inner<T>>` — not three methods bolted onto every `Arc` — and
//! `impl<T: Display> ToString for T` is written for nothing in particular and
//! has to be tried against whatever the receiver turned out to be.

use std::collections::HashMap;

use super::{MethodSig, TypeRegistry};
use crate::ty::subst::Subst;
use crate::ty::{unify, Prim, TraitRef, Ty, TypeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImplId(pub u32);

/// One `T: Trait` requirement, written inline on the impl's generics or in its
/// `where` clause. The two mean the same thing and are kept together.
#[derive(Debug, Clone, PartialEq)]
pub struct Bound {
    pub subject: Ty,
    pub trait_ref: TraitRef,
}

/// An `impl` block: what it is for, what it needs, and what it supplies.
#[derive(Debug, Clone)]
pub struct ImplDef {
    pub id: ImplId,
    /// The parameters the impl declares. These, and only these, are the
    /// unknowns when `self_ty` is matched against a receiver.
    pub generics: Vec<String>,
    pub bounds: Vec<Bound>,
    pub self_ty: Ty,
    /// `None` for an inherent impl.
    pub trait_ref: Option<TraitRef>,
    /// `type Target = [Attestation];` — what this impl supplies for the trait's
    /// associated types.
    pub assoc_types: HashMap<String, Ty>,
    pub methods: HashMap<String, MethodSig>,
}

impl ImplDef {
    pub fn is_inherent(&self) -> bool {
        self.trait_ref.is_none()
    }

    /// Is this written for one of its own parameters — `impl<T: Display> ToString
    /// for T` — so that it applies to whatever the receiver turns out to be?
    pub fn is_blanket(&self) -> bool {
        matches!(&self.self_ty, Ty::Param(name) if self.generics.iter().any(|g| g == name))
    }

    /// Does this impl apply to `concrete`, and if so what does it bind its own
    /// parameters to?
    pub fn match_self(&self, concrete: &Ty) -> Option<Subst> {
        self.match_pattern(&self.self_ty, concrete)
    }

    /// Match one of this impl's written types against a concrete one.
    pub fn match_written(&self, pattern: &Ty, concrete: &Ty) -> Option<Subst> {
        self.match_pattern(pattern, concrete)
    }

    /// Match the trait this impl implements, argument by argument, against the
    /// trait reference a bound requires: `impl<T> FromIterator<T> for Vec<T>`
    /// against `FromIterator<Listener>` binds `T`.
    pub fn match_written_args(&self, implemented: &TraitRef, want: &TraitRef) -> Option<Subst> {
        if implemented.id != want.id || implemented.args.len() != want.args.len() {
            return None;
        }
        let mut subst = Subst::new();
        for (pattern, concrete) in implemented.args.iter().zip(&want.args) {
            let found = self.match_pattern(pattern, concrete)?;
            for (name, ty) in found {
                if subst.get(&name).is_some_and(|existing| *existing != ty) {
                    return None;
                }
                subst.insert(name, ty);
            }
        }
        Some(subst)
    }

    /// Match one of this impl's written types against a concrete one, with the
    /// impl's parameters renamed apart first.
    ///
    /// `impl<T> Deref for Arc<T>` matched against `Arc<RwLock<T>>` is an
    /// ordinary match, but the two `T`s are different parameters that happen to
    /// share a name, and binding one to a type containing the other would look
    /// like a parameter containing itself. Rust tells them apart by which binder
    /// declared them; the engine has only names, so it makes fresh ones for the
    /// length of the match. The bindings handed back are keyed by the impl's own
    /// names and never mention a fresh one, because a variable can only ever be
    /// bound to a piece of `concrete`.
    fn match_pattern(&self, pattern: &Ty, concrete: &Ty) -> Option<Subst> {
        let mut subst = Subst::new();
        if self.generics.is_empty() {
            unify(&[], pattern, concrete, &mut subst).ok()?;
            return Some(subst);
        }
        let fresh: Vec<String> = self
            .generics
            .iter()
            .map(|g| format!("{}#{}", g, self.id.0))
            .collect();
        let rename: Subst = self
            .generics
            .iter()
            .cloned()
            .zip(fresh.iter().map(|f| Ty::Param(f.clone())))
            .collect();
        let pattern = pattern.substitute(&rename);
        unify(&fresh, &pattern, concrete, &mut subst).ok()?;
        Some(
            self.generics
                .iter()
                .zip(&fresh)
                .filter_map(|(name, f)| subst.get(f).map(|ty| (name.clone(), ty.clone())))
                .collect(),
        )
    }
}

/// How an impl's self type is filed, so that a receiver only has to be matched
/// against the impls that could possibly be for it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Head {
    Named(TypeId),
    Prim(Prim),
    Ref,
    Tuple(usize),
    Slice,
    Array,
    Str,
    Unit,
    Never,
    Dyn,
    /// A projection, or a parameter the impl did not declare: nothing to file it
    /// under, so it is tried against every receiver.
    Open,
}

pub fn head_of(ty: &Ty, generics: &[String]) -> Head {
    match ty {
        Ty::Named { id, .. } => Head::Named(*id),
        Ty::Prim(p) => Head::Prim(*p),
        Ty::Ref { .. } => Head::Ref,
        Ty::Tuple(elems) => Head::Tuple(elems.len()),
        Ty::Slice(_) => Head::Slice,
        Ty::Array { .. } => Head::Array,
        Ty::Str => Head::Str,
        Ty::Unit => Head::Unit,
        Ty::Never => Head::Never,
        Ty::Dyn { .. } => Head::Dyn,
        Ty::Param(name) if generics.iter().any(|g| g == name) => Head::Open,
        Ty::Param(_) | Ty::Assoc { .. } | Ty::ImplTrait { .. } | Ty::Infer => Head::Open,
    }
}

#[derive(Debug, Default)]
pub struct ImplTable {
    impls: Vec<ImplDef>,
    /// Impls written for a definite shape, by that shape.
    by_head: HashMap<Head, Vec<ImplId>>,
    /// Impls of a trait, by the trait, so `Deref` and `From` can be asked for
    /// directly.
    by_trait: HashMap<TypeId, Vec<ImplId>>,
    /// Impls written for one of their own parameters.
    blanket: Vec<ImplId>,
    /// The blanket impls that offer each method name, filled in once every
    /// trait has its declarations. A blanket impl matches *every* receiver, so
    /// without this each method call in the corpus unified against all of them
    /// — and the surface declares hundreds.
    blanket_by_method: HashMap<String, Vec<ImplId>>,
}

impl ImplTable {
    pub fn push(&mut self, mut def: ImplDef) -> ImplId {
        let id = ImplId(self.impls.len() as u32);
        def.id = id;
        if let Some(tr) = &def.trait_ref {
            self.by_trait.entry(tr.id).or_default().push(id);
        }
        if def.is_blanket() {
            self.blanket.push(id);
        } else {
            let head = head_of(&def.self_ty, &def.generics);
            self.by_head.entry(head).or_default().push(id);
        }
        self.impls.push(def);
        id
    }

    pub fn get(&self, id: ImplId) -> &ImplDef {
        &self.impls[id.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.impls.len()
    }

    /// Every impl that could be for a receiver of this shape: the ones filed
    /// under it, plus the ones filed under nothing.
    pub fn for_head(&self, head: &Head) -> impl Iterator<Item = ImplId> + '_ {
        let exact = self.by_head.get(head).map(|v| v.as_slice()).unwrap_or(&[]);
        let open = self
            .by_head
            .get(&Head::Open)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        exact.iter().copied().chain(open.iter().copied())
    }

    pub fn blanket(&self) -> &[ImplId] {
        &self.blanket
    }

    /// The blanket impls that could answer to this method name.
    ///
    /// Before the index is built — during the passes that resolve
    /// declarations — every blanket is a candidate, because a trait whose
    /// methods are not yet known cannot say what it offers.
    pub fn blanket_offering(&self, name: &str) -> &[ImplId] {
        if self.blanket_by_method.is_empty() {
            return &self.blanket;
        }
        self.blanket_by_method
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub(super) fn set_blanket_index(&mut self, index: HashMap<String, Vec<ImplId>>) {
        self.blanket_by_method = index;
    }

    pub fn of_trait(&self, trait_id: TypeId) -> &[ImplId] {
        self.by_trait
            .get(&trait_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn all(&self) -> impl Iterator<Item = &ImplDef> {
        self.impls.iter()
    }
}

impl TypeRegistry {
    pub fn impls(&self) -> &ImplTable {
        &self.impls
    }

    pub fn add_impl(&mut self, def: ImplDef) -> ImplId {
        self.impls.push(def)
    }

    pub fn impl_def(&self, id: ImplId) -> &ImplDef {
        self.impls.get(id)
    }
}
