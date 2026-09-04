//! Saying what the engine worked out, in words a person can read.
//!
//! A diagnostic is read next to the Rust source it came from, so it names types
//! and functions the way Rust writes them — `&HashMap<usize, Listener<T>>`,
//! `<Broadcast<T> as Clone>::clone` — rather than by their ids. The oracle test
//! reads the same rendering, which is why it lives in one place.

use super::method::{Callee, MethodError};
use super::TypeRegistry;
use crate::ty::{TraitRef, Ty};

impl TypeRegistry {
    /// A type in words, for a diagnostic a person reads next to the Rust source.
    pub fn describe(&self, ty: &Ty) -> String {
        match ty {
            Ty::Prim(p) => format!("{:?}", p).to_lowercase(),
            Ty::Named { id, args } => {
                let name = self.name_of(*id);
                if args.is_empty() {
                    name
                } else {
                    format!(
                        "{}<{}>",
                        name,
                        args.iter()
                            .map(|a| self.describe(a))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            Ty::Param(name) => name.clone(),
            Ty::ImplTrait { bounds } => format!("impl {}", self.describe_traits(bounds)),
            Ty::Ref { mutable, inner } => {
                let m = if *mutable { "mut " } else { "" };
                format!("&{}{}", m, self.describe(inner))
            }
            Ty::Tuple(elems) => format!(
                "({})",
                elems
                    .iter()
                    .map(|e| self.describe(e))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Ty::Array { elem, .. } => format!("[{}; _]", self.describe(elem)),
            Ty::Slice(elem) => format!("[{}]", self.describe(elem)),
            Ty::Str => "str".to_string(),
            Ty::Unit => "()".to_string(),
            Ty::Never => "!".to_string(),
            Ty::Dyn { traits } => format!("dyn {}", self.describe_traits(traits)),
            Ty::Assoc { base, name, .. } => format!("{}::{}", self.describe(base), name),
            Ty::Infer => "_".to_string(),
        }
    }

    fn describe_traits(&self, traits: &[TraitRef]) -> String {
        traits
            .iter()
            .map(|t| self.name_of(t.id))
            .collect::<Vec<_>>()
            .join(" + ")
    }

    /// The callee in words: which impl, and which trait it came through.
    pub fn describe_callee(&self, callee: &Callee) -> String {
        match callee {
            Callee::Inherent(id, m) => {
                format!("{}::{}", self.describe(&self.impl_def(*id).self_ty), m)
            }
            Callee::TraitImpl(id, m) | Callee::Blanket(id, m) => {
                let def = self.impl_def(*id);
                let trait_name = def
                    .trait_ref
                    .as_ref()
                    .map(|t| self.name_of(t.id))
                    .unwrap_or_default();
                format!("<{} as {}>::{}", self.describe(&def.self_ty), trait_name, m)
            }
            Callee::TraitObject(id, m) => format!("{}::{}", self.name_of(*id), m),
        }
    }
}

impl MethodError {
    /// The diagnostic this failure prints, naming what was tried.
    pub fn describe(&self, reg: &TypeRegistry, method: &str) -> String {
        match self {
            MethodError::NotFound { receiver, tried } => format!(
                "no method `{}` on `{}`; tried {}",
                method,
                reg.describe(receiver),
                tried
                    .iter()
                    .map(|t| format!("`{}`", reg.describe(t)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            MethodError::Ambiguous { at, candidates } => format!(
                "`{}` on `{}` is ambiguous between {}",
                method,
                reg.describe(at),
                candidates
                    .iter()
                    .map(|c| reg.describe_callee(c))
                    .collect::<Vec<_>>()
                    .join(" and ")
            ),
            MethodError::DerefCycle { receiver } => format!(
                "`{}` dereferences without end, so `{}` cannot be looked for",
                reg.describe(receiver),
                method
            ),
        }
    }
}

