//! What a trait declares.
//!
//! A trait is the only thing that can answer a call on a `dyn Trait`, on a
//! generic parameter bounded by it, and on `Self` inside one of its own default
//! method bodies. It is also where a trait impl's method signature comes from
//! when the impl inherits a default body rather than writing one.

use std::collections::HashMap;

use super::{MethodSig, TypeRegistry};
use crate::ty::{TraitRef, TypeId};

/// One method a trait declares.
#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub sig: MethodSig,
}

/// A trait, keyed in the registry by the same `TypeId` its name resolves to.
#[derive(Debug, Clone)]
pub struct TraitDef {
    pub id: TypeId,
    /// The parameters the trait itself declares, which its method signatures
    /// are written in terms of.
    pub generics: Vec<String>,
    /// `trait Signal: Debug` — an implementor implements these too, so a method
    /// reached on a `dyn Signal` may be declared on one of them.
    pub supertraits: Vec<TraitRef>,
    /// `type Item;` — the names each impl has to supply a type for. A
    /// projection on a `dyn Trait` or a bounded parameter is only meaningful
    /// when the trait declares that name, which is what this answers.
    pub assoc_types: Vec<String>,
    pub methods: HashMap<String, TraitMethod>,
    /// `auto trait Send {}`. Rust works these out from a type's fields rather
    /// than from an impl, so there is nothing in the impl table to find and a
    /// bound on one is proved by the declaration alone. The corpus compiles
    /// under rustc, so every auto-trait bound it writes already holds.
    pub is_auto: bool,
}

impl TypeRegistry {
    pub fn trait_def(&self, id: TypeId) -> Option<&TraitDef> {
        self.traits.get(&id)
    }

    pub(super) fn insert_trait(&mut self, def: TraitDef) {
        self.traits.insert(def.id, def);
    }

    /// A method this trait itself declares, ignoring its supertraits.
    ///
    /// An `impl ExactSizeIterator for Values` supplies `ExactSizeIterator`'s
    /// methods and no others: `Iterator::cloned` is supplied by the `Iterator`
    /// impl, and letting the subtrait's impl offer it too made every such call
    /// ambiguous between two impls of one method.
    pub fn trait_own_method(&self, trait_id: TypeId, name: &str) -> Option<&TraitMethod> {
        self.trait_def(trait_id)?.methods.get(name)
    }

    /// A trait's own method declaration, then its supertraits', innermost
    /// first. Used by `dyn Trait` dispatch and by a call on a bounded parameter.
    ///
    /// Returns the trait the declaration was found on together with the method,
    /// because the two differ whenever a supertrait supplies it.
    pub fn trait_method(&self, trait_id: TypeId, name: &str) -> Option<(TypeId, &TraitMethod)> {
        self.trait_method_within(trait_id, name, &mut Vec::new())
    }

    fn trait_method_within(
        &self,
        trait_id: TypeId,
        name: &str,
        seen: &mut Vec<TypeId>,
    ) -> Option<(TypeId, &TraitMethod)> {
        if seen.contains(&trait_id) {
            return None;
        }
        seen.push(trait_id);
        let def = self.trait_def(trait_id)?;
        if let Some(method) = def.methods.get(name) {
            return Some((trait_id, method));
        }
        // Cloned out of the borrow so the recursive call can borrow `self` again.
        let supers: Vec<TypeId> = def.supertraits.iter().map(|t| t.id).collect();
        for supertrait in supers {
            if let Some(found) = self.trait_method_within(supertrait, name, seen) {
                return Some(found);
            }
        }
        None
    }
}
