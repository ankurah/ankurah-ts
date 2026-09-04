//! Substituting concrete types for type parameters.
//!
//! `RwLock<T>` declares `write(&self) -> RwLockWriteGuard<T>`. Calling it on a
//! `RwLock<HashMap<K, V>>` binds `T` to `HashMap<K, V>` and rewrites the return
//! type through that binding.

use std::collections::HashMap;

use super::def::{TraitRef, Ty};

/// Type parameter name to the type standing in for it.
pub type Subst = HashMap<String, Ty>;

impl Ty {
    pub fn substitute(&self, subst: &Subst) -> Ty {
        if subst.is_empty() {
            return self.clone();
        }
        match self {
            Ty::Param(name) => subst.get(name).cloned().unwrap_or_else(|| self.clone()),
            Ty::Named { id, args } => Ty::Named {
                id: *id,
                args: args.iter().map(|a| a.substitute(subst)).collect(),
            },
            Ty::ImplTrait { bounds } => Ty::ImplTrait {
                bounds: bounds.iter().map(|b| b.substitute(subst)).collect(),
            },
            Ty::Ref { mutable, inner } => Ty::Ref {
                mutable: *mutable,
                inner: Box::new(inner.substitute(subst)),
            },
            Ty::Tuple(elems) => Ty::Tuple(elems.iter().map(|e| e.substitute(subst)).collect()),
            Ty::Array { elem, len } => Ty::Array {
                elem: Box::new(elem.substitute(subst)),
                len: len.clone(),
            },
            Ty::Slice(inner) => Ty::Slice(Box::new(inner.substitute(subst))),
            Ty::Dyn { traits } => Ty::Dyn {
                traits: traits.iter().map(|t| t.substitute(subst)).collect(),
            },
            Ty::Assoc { base, trait_, name } => Ty::Assoc {
                base: Box::new(base.substitute(subst)),
                trait_: trait_.as_ref().map(|t| Box::new(t.substitute(subst))),
                name: name.clone(),
            },
            Ty::Prim(_) | Ty::Str | Ty::Unit | Ty::Never | Ty::Infer => self.clone(),
        }
    }
}

impl TraitRef {
    pub fn substitute(&self, subst: &Subst) -> TraitRef {
        TraitRef {
            id: self.id,
            args: self.args.iter().map(|a| a.substitute(subst)).collect(),
            bindings: self
                .bindings
                .iter()
                .map(|(n, t)| (n.clone(), t.substitute(subst)))
                .collect(),
        }
    }
}

/// Pair a type's declared parameters with the arguments actually written.
/// Missing arguments simply leave that parameter unbound, which keeps a bare
/// `Foo` usable where `Foo<T>` was declared.
pub fn bind_params(type_params: &[String], args: &[Ty]) -> Subst {
    type_params
        .iter()
        .cloned()
        .zip(args.iter().cloned())
        .collect()
}
