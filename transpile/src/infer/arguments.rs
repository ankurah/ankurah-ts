//! What each ARGUMENT of a free call stands at, and what the call itself fixes.
//!
//! Split out of `calls.rs`. Two questions live here and they are asked in one
//! order: what the written arguments settle of the callee's own parameters, and
//! then what each parameter type is once they have. Asked the other way round,
//! `generic_apply<T, F: Fn(T) -> u32>(v: T, f: F)` read `F: Fn(T)` with `T`
//! still open and typed the closure's parameter as the callee's own — "no field
//! `n` on `T`", and a value handed to the closure BY VALUE that nothing
//! released (GG6/FF7).

use super::calls::unparenthesise;
use super::context::{open_params, TypeContext};
use super::expected;
use crate::ty::{subst::Subst, Ty};

impl TypeContext<'_> {
    /// Bind what the written arguments settle of the callee's own parameters.
    ///
    /// Only the arguments this body can type, and only where the binding is new:
    /// what the expected return type fixed is the caller's own claim and wins.
    /// A closure is skipped, because its own type is what the bound is about to
    /// say.
    fn fixed_by_the_actuals(
        &self,
        call: &syn::ExprCall,
        sig: &crate::registry::MethodSig,
        subst: &mut crate::ty::subst::Subst,
    ) {
        let vars: Vec<String> = sig.type_params.clone();
        if vars.is_empty() {
            return;
        }
        for (index, (_, declared)) in sig.params.iter().enumerate() {
            let Some(written) = call.args.iter().nth(index) else { continue };
            if matches!(unparenthesise(written), syn::Expr::Closure(_)) {
                continue;
            }
            if open_params(&declared.substitute(subst)).is_empty() {
                continue;
            }
            let mark = self.sink.mark();
            let actual = self.resolve_expr(written);
            self.sink.rewind(mark);
            let Ok(actual) = actual else { continue };
            let mut learned = crate::ty::subst::Subst::default();
            if crate::ty::unify(&vars, declared, &actual, &mut learned).is_ok() {
                for (name, ty) in learned.iter() {
                    subst.entry(name.clone()).or_insert_with(|| ty.clone());
                }
            }
        }
    }

    pub(super) fn call_argument_types_of(
        &self,
        call: &syn::ExprCall,
        expected: Option<&Ty>,
    ) -> Option<Vec<Option<Ty>>> {
        let syn::Expr::Path(path) = &*call.func else {
            return None;
        };
        if let Some(fields) = self.variant_argument_types(path, expected) {
            return Some(fields);
        }
        if let Some(fields) = self.tuple_struct_argument_types(path, expected) {
            return Some(fields);
        }
        let (sig, mut subst) = self.call_sig(call, expected)?;
        let probe = self.probe();
        // GG6/FF7: what the SIBLING actuals fix, before the callable bounds are
        // read. `call_sig` binds only what the expected RETURN type settled, so
        // `generic_apply<T, F: Fn(T) -> u32>(v: T, f: F)` called as
        // `generic_apply(token, |x| ..)` read `F: Fn(T)` with `T` still open
        // and typed `x` as the callee's own parameter — "no field `n` on `T`",
        // and a value handed to the closure BY VALUE that nothing released. A
        // written argument the engine can type says what the parameter beside
        // it is; a closure says nothing until the bound is read, so it is not
        // asked here.
        self.fixed_by_the_actuals(call, &sig, &mut subst);
        // The callee's own bounds, with whatever this call fixed of them
        // substituted in. They are what says what a CLOSURE at one of these
        // positions has to be, and the bounds this body carries cannot: the
        // bound belongs to the callee's generics.
        let callee_bounds: Vec<(String, crate::ty::TraitRef)> =
            crate::registry::method::param_bounds_of(&sig.bounds)
                .into_iter()
                .map(|(subject, bound)| (subject, bound.substitute(&subst)))
                .collect();
        let written: Vec<&syn::Expr> = call.args.iter().collect();
        Some(
            sig.params
                .iter()
                .enumerate()
                .map(|(index, (_, ty))| {
                    let filled = probe.normalize(&ty.substitute(&subst));
                    if !expected::has_infer(&filled) && open_params(&filled).is_empty() {
                        return Some(filled);
                    }
                    // A closure at a parameter the callee bounded by `Fn`
                    // takes its parameter types from that bound.
                    expected::callable_bound_for(
                        self.registry,
                        written.get(index).copied(),
                        &filled,
                        &callee_bounds,
                    )
                })
                .collect(),
        )
    }

    /// What a TUPLE STRUCT's constructor takes: its fields, in order.
    ///
    /// `Clock(ids.into_iter().collect())` is a call whose callee is a type, not
    /// a function, and nothing declared what it takes — so the `collect` inside
    /// it had no target and became a hole. Rust's tuple struct is exactly a
    /// constructor over its field types, and the registry has them.
    ///
    /// A named-field struct is not this: `Wrap { items: .. }` is a struct
    /// LITERAL, whose fields are typed where the literal is written.
    pub(super) fn tuple_struct_argument_types(
        &self,
        path: &syn::ExprPath,
        expected: Option<&Ty>,
    ) -> Option<Vec<Option<Ty>>> {
        let segments: Vec<String> = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        // `Self(..)` inside the type's own impl names the same constructor:
        // `impl From<Vec<EventId>> for Clock { fn from(ids) -> Self {
        // Self(ids.into_iter().collect()) } }` is `proto`'s, and the `collect`
        // inside it had no target.
        let id = if segments == ["Self"] {
            match self.self_ty.as_ref() {
                Some(Ty::Named { id, .. }) => *id,
                _ => return None,
            }
        } else {
            match self.registry.lookup(self.module, crate::registry::Ns::Type, &segments).ok()?? {
                crate::registry::Def::Type(id) => id,
                _ => return None,
            }
        };
        let def = self.registry.def(id)?;
        if !matches!(def.kind, crate::registry::TypeKind::Struct) {
            return None;
        }
        // Positional fields are named `_0`, `_1` — the spelling emission uses
        // and the one `field_order` records for a tuple struct.
        if def.field_order.is_empty() || !def.field_order.iter().all(|f| f.starts_with('_')) {
            return None;
        }
        // The position binds whatever the declaration left open: `Wrapper<T>`
        // in a place wanting a `Wrapper<u8>` takes a `u8`.
        let mut subst = Subst::new();
        if let Some(Ty::Named { id: want, args }) = expected.map(|ty| ty.peel_refs()) {
            if *want == id {
                for (param, arg) in def.type_params.iter().zip(args) {
                    subst.insert(param.clone(), arg.clone());
                }
            }
        }
        Some(
            def.field_order
                .iter()
                .map(|name| {
                    let ty = def.fields.iter().find(|(f, _)| f == name)?;
                    let filled = ty.1.substitute(&subst);
                    (open_params(&filled).is_empty() && !expected::has_infer(&filled)).then_some(filled)
                })
                .collect(),
        )
    }
}
