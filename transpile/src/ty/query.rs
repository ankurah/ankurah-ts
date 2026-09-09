//! What a type says about itself.
//!
//! One place answers "does this mention X?" for every consumer, so a new `Ty`
//! variant is judged once rather than once per asker. Unification, the body
//! solver and bound lookup all read these.

use super::def::{InferId, TraitRef, Ty};

impl Ty {
    /// Does this type mention the parameter by that name anywhere inside it?
    pub fn mentions_param(&self, name: &str) -> bool {
        match self {
            Ty::Param(p) => p == name,
            Ty::Named { args, .. } | Ty::Tuple(args) => {
                args.iter().any(|a| a.mentions_param(name))
            }
            Ty::Ref { inner, .. } | Ty::Slice(inner) | Ty::Array { elem: inner, .. } => {
                inner.mentions_param(name)
            }
            Ty::Dyn { traits } | Ty::ImplTrait { bounds: traits } => {
                traits.iter().any(|t| t.mentions_param(name))
            }
            Ty::Assoc { base, trait_, .. } => {
                base.mentions_param(name)
                    || trait_.as_ref().is_some_and(|t| t.mentions_param(name))
            }
            Ty::Prim(_) | Ty::Str | Ty::Unit | Ty::Never | Ty::Infer | Ty::Var(_) => false,
        }
    }
}

impl Ty {
    /// Is there a `_` anywhere inside this type?
    pub fn mentions_infer(&self) -> bool {
        match self {
            Ty::Infer => true,
            Ty::Named { args, .. } | Ty::Tuple(args) => args.iter().any(|a| a.mentions_infer()),
            Ty::Ref { inner, .. } | Ty::Slice(inner) | Ty::Array { elem: inner, .. } => {
                inner.mentions_infer()
            }
            Ty::Dyn { traits } | Ty::ImplTrait { bounds: traits } => {
                traits.iter().any(|t| t.mentions_infer())
            }
            Ty::Assoc { base, .. } => base.mentions_infer(),
            Ty::Param(_) | Ty::Prim(_) | Ty::Str | Ty::Unit | Ty::Never | Ty::Var(_) => false,
        }
    }

    /// Is this inference variable mentioned anywhere inside this type? The
    /// occurs check asks it: `?0 = Vec<?0>` names no type.
    pub fn mentions_var(&self, var: InferId) -> bool {
        self.any_var(&mut |id| id == var)
    }

    /// Is any inference variable mentioned inside this type?
    pub fn mentions_any_var(&self) -> bool {
        self.any_var(&mut |_| true)
    }

    /// Does an unread projection stand anywhere inside this type?
    ///
    /// `I::IntoIter::Item` and `F` are one type wherever a `where` clause says
    /// so, and that clause is read through the impl table, which cannot read it
    /// for a parameter nothing has instantiated. A constraint over such a type
    /// is evidence neither way.
    pub fn mentions_projection(&self) -> bool {
        match self {
            Ty::Assoc { .. } => true,
            Ty::Named { args, .. } | Ty::Tuple(args) => {
                args.iter().any(|a| a.mentions_projection())
            }
            Ty::Ref { inner, .. } | Ty::Slice(inner) | Ty::Array { elem: inner, .. } => {
                inner.mentions_projection()
            }
            Ty::Dyn { traits } | Ty::ImplTrait { bounds: traits } => traits
                .iter()
                .any(|t| t.args.iter().any(|a| a.mentions_projection())),
            Ty::Param(_) | Ty::Prim(_) | Ty::Str | Ty::Unit | Ty::Never | Ty::Infer | Ty::Var(_) => {
                false
            }
        }
    }

    fn any_var(&self, pred: &mut impl FnMut(InferId) -> bool) -> bool {
        match self {
            Ty::Var(id) => pred(*id),
            Ty::Named { args, .. } | Ty::Tuple(args) => args.iter().any(|a| a.any_var(pred)),
            Ty::Ref { inner, .. } | Ty::Slice(inner) | Ty::Array { elem: inner, .. } => {
                inner.any_var(pred)
            }
            Ty::Dyn { traits } | Ty::ImplTrait { bounds: traits } => {
                traits.iter().any(|t| t.any_var(pred))
            }
            Ty::Assoc { base, trait_, .. } => {
                base.any_var(pred) || trait_.as_ref().is_some_and(|t| t.any_var(pred))
            }
            Ty::Param(_) | Ty::Prim(_) | Ty::Str | Ty::Unit | Ty::Never | Ty::Infer => false,
        }
    }
}

impl TraitRef {
    fn any_var(&self, pred: &mut impl FnMut(InferId) -> bool) -> bool {
        self.args.iter().any(|a| a.any_var(pred))
            || self.bindings.iter().any(|(_, t)| t.any_var(pred))
    }

    pub fn mentions_infer(&self) -> bool {
        self.args.iter().any(|a| a.mentions_infer())
            || self.bindings.iter().any(|(_, t)| t.mentions_infer())
    }

    pub fn mentions_param(&self, name: &str) -> bool {
        self.args.iter().any(|a| a.mentions_param(name))
            || self.bindings.iter().any(|(_, t)| t.mentions_param(name))
    }
}

impl Ty {
    /// Does a hole a turbofish left — `Vec<_>` — appear anywhere in this type?
    /// A method's own bound is what fills one. An inference variable is not
    /// one: the body's solver answers it, and answers it from the table.
    pub fn contains_infer(&self) -> bool {
        match self {
            Ty::Infer => true,
            Ty::Var(_) => false,
            Ty::Named { args, .. } | Ty::Tuple(args) => args.iter().any(|a| a.contains_infer()),
            Ty::Ref { inner, .. } | Ty::Slice(inner) | Ty::Array { elem: inner, .. } => {
                inner.contains_infer()
            }
            Ty::Assoc { base, .. } => base.contains_infer(),
            Ty::Param(_) | Ty::Dyn { .. } | Ty::ImplTrait { .. } => false,
            Ty::Prim(_) | Ty::Str | Ty::Unit | Ty::Never => false,
        }
    }

    /// Does any type parameter or unknown survive inside this type? A bound on
    /// such a type cannot be looked up, because there is no type yet to look it
    /// up for.
    pub fn has_open_param(&self) -> bool {
        match self {
            Ty::Param(_) | Ty::Infer | Ty::Var(_) => true,
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
