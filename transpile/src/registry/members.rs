//! Reading a field or a method off a type.
//!
//! Step 1 keeps the single-hop deref this replaced: a wrapper type names the
//! TypeScript accessor to emit and the search continues into its first type
//! argument. The impl-table step replaces both with a real deref chain built
//! from `Deref` impls, which is what the nested and unsize cases need.

use super::{MethodSig, TypeKind, TypeRegistry};
use crate::ty::subst::Subst;
use crate::ty::{bind_params, Ty};

impl TypeRegistry {
    /// The type of `expr.field`, and the accessor to emit before it.
    ///   `None`          — read the field directly
    ///   `Some("")`      — transparent wrapper, emit nothing
    ///   `Some("value")` — emit `.value` first
    pub fn resolve_field(&self, ty: &Ty, field: &str) -> Option<(Ty, Option<String>)> {
        let Ty::Named { id, args } = ty.peel_refs() else {
            return None;
        };
        let def = self.def(*id)?;
        let subst = bind_params(&def.type_params, args);

        for (name, field_ty) in &def.fields {
            if name == field {
                return Some((field_ty.substitute(&subst), None));
            }
        }

        let accessor = def.deref_field.as_ref()?;
        let inner = args.first()?;
        let (resolved, inner_accessor) = self.resolve_field(inner, field)?;
        let emitted = if accessor.is_empty() {
            inner_accessor
        } else {
            Some(accessor.clone())
        };
        Some((resolved, emitted))
    }

    /// The return type of `expr.method(..)`, with the receiver's type
    /// arguments substituted in.
    pub fn resolve_method(&self, ty: &Ty, method: &str) -> Option<Ty> {
        let Ty::Named { id, args } = ty.peel_refs() else {
            return None;
        };
        let def = self.def(*id)?;

        if let Some(sig) = def.methods.get(method) {
            return Some(sig.ret.substitute(&receiver_subst(sig, args)));
        }

        if def.deref_field.is_some() {
            return self.resolve_method(args.first()?, method);
        }
        None
    }

    /// Is this method declared on the type itself rather than reached through
    /// its wrapper? `arc.clone()` must not become `arc.value.clone()`.
    pub fn is_own_method(&self, ty: &Ty, method: &str) -> bool {
        match ty.peel_refs() {
            Ty::Named { id, .. } => self
                .def(*id)
                .map(|d| d.methods.contains_key(method))
                .unwrap_or(false),
            _ => false,
        }
    }

    /// The accessor that reaches through a wrapper type, if it is one.
    pub fn deref_field(&self, ty: &Ty) -> Option<&str> {
        match ty.peel_refs() {
            Ty::Named { id, .. } => self.def(*id)?.deref_field.as_deref(),
            _ => None,
        }
    }

    /// The type behind a wrapper: the first type argument.
    pub fn deref_target(&self, ty: &Ty) -> Option<Ty> {
        match ty.peel_refs() {
            Ty::Named { args, .. } => args.first().cloned(),
            _ => None,
        }
    }

    pub fn is_variant_of(&self, id: crate::ty::TypeId, variant: &str) -> bool {
        match self.def(id).map(|d| &d.kind) {
            Some(TypeKind::Enum { variants }) => variants.iter().any(|v| v.name == variant),
            _ => false,
        }
    }
}

/// Bind the receiver's type arguments to the names the impl block wrote for
/// them. `impl<E> Wrap<E>` on `struct Wrap<R>` returns `E`, so substituting
/// through the struct's `R` would leave the return type dangling.
fn receiver_subst(sig: &MethodSig, args: &[Ty]) -> Subst {
    sig.receiver_params
        .iter()
        .zip(args.iter())
        .filter_map(|(name, arg)| name.clone().map(|n| (n, arg.clone())))
        .collect()
}
