//! What a CALL resolves to, and what it wants of its arguments.
//!
//! For: the type of a call is the callee's return type with the substitution
//! the receiver and the turbofish imply, and finding the callee is the whole of
//! method resolution — through the impl table, through a bound on a type
//! parameter, through a trait's own declaration. The other half is the
//! question the translator asks BEFORE it writes the arguments: what does the
//! callee declare each one to be? That is what types a closure the signature
//! alone says nothing about, and what settles the width of an unsuffixed
//! literal handed to a `u8` parameter.
//!
//! `variant_type` and `variant_argument_types` are here rather than with the
//! struct literals because `Some(x)` and `Ok(e.into())` are written as calls
//! and resolve as calls.

use syn::spanned::Spanned;

use super::context::{open_params, TypeContext};
use super::expected;
use crate::diag::Diag;
use crate::registry::{Def, FieldResolution, MethodResolution, Ns};
use crate::ty::subst::Subst;
use crate::ty::{unify, Ty};

impl TypeContext<'_> {
    // ── Calls ──────────────────────────────────────────────────────────

    /// Which function `receiver.name(..)` calls, and what it hands back.

    /// The same, with the type arguments a turbofish wrote. `collect::<Vec<_>>()`
    /// says what the call produces where nothing else does.
    pub fn resolve_method_call_with(
        &self,
        receiver: &syn::Expr,
        method: &str,
        turbofish: Option<&syn::AngleBracketedGenericArguments>,
    ) -> Result<MethodResolution, Diag> {
        // `&x` as an EXPRESSION types as `x` — emission erases borrows, and
        // every reader downstream is written against the value. As a RECEIVER
        // it does not: Rust's probe starts at `&Vec<T>` and finds
        // `impl IntoIterator for &'a Vec<T>`, whose `Item` is `&T`, where
        // starting at `Vec<T>` finds the by-value impl and an owned `Item`.
        // `(&v).into_iter()` therefore came out as a loop that released the
        // caller's elements — a double drop where the block released them too
        // (E11). The borrow is put back here and nowhere else, so only the
        // probe sees it; the deref chain takes it straight off again, which is
        // what Rust does when the method really is the by-value one.
        let receiver_ty = match unparenthesise(receiver) {
            syn::Expr::Reference(r) => Ty::Ref {
                mutable: r.mutability.is_some(),
                inner: Box::new(self.resolve_expr(&r.expr)?),
            },
            _ => self.resolve_expr(receiver)?,
        };
        let probe = self.probe();

        let mut explicit = Vec::new();
        for arg in turbofish.iter().flat_map(|t| t.args.iter()) {
            match arg {
                syn::GenericArgument::Type(ty) => explicit.push(self.resolve_written_type(ty)?),
                syn::GenericArgument::Lifetime(_) => {}
                other => {
                    return Err(self.refuse(
                        syn::spanned::Spanned::span(other),
                        "only type arguments are read from a turbofish",
                    ))
                }
            }
        }

        let found = probe
            .resolve_method_with(&receiver_ty, method, &explicit)
            .map_err(|err| self.refuse(receiver.span(), err.describe(self.registry, method)))?;

        // A resolution that rests on a question nobody answered is still an
        // answer the translator uses, so the question is filed where the call is.
        // Step 4 discharges these (spec 4.5); until it does they are part of the
        // measure of what the engine cannot yet decide.
        for obligation in &found.obligations {
            self.sink.report(
                receiver.span(),
                format!(
                    "obligation deferred: `{}: {}` ({})",
                    self.registry.describe(&obligation.subject),
                    self.registry.name_of(obligation.bound.id),
                    match obligation.reason {
                        crate::registry::Undecided::NoDeclaration =>
                            "the trait has no declaration here",
                        crate::registry::Undecided::OpenSubject =>
                            "the subject is still a type parameter",
                        crate::registry::Undecided::DepthLimit => "the search ran too deep",
                    }
                ),
            );
        }
        if let Some(trait_id) = found.out_of_scope {
            self.sink.report(
                receiver.span(),
                format!(
                    "method `{}` resolved through trait `{}`, which is not in scope here",
                    method,
                    self.registry.name_of(trait_id)
                ),
            );
        }
        Ok(found)
    }

    /// Where a field lives, and what has to be written to reach it.
    pub fn resolve_field_access(
        &self,
        base: &syn::Expr,
        member: &str,
    ) -> Result<FieldResolution, Diag> {
        let base_ty = self.resolve_expr(base)?;
        self.probe().resolve_field(&base_ty, member).ok_or_else(|| {
            self.refuse(
                base.span(),
                format!(
                    "no field `{}` on `{}`",
                    member,
                    self.registry.describe(&base_ty)
                ),
            )
        })
    }

    /// A `Type::function(..)`, an enum variant carrying a payload, or a plain
    /// function call.
    pub(super) fn resolve_call(&self, call: &syn::ExprCall) -> Result<Ty, Diag> {
        let syn::Expr::Path(path) = &*call.func else {
            return Err(self.refuse(
                call.func.span(),
                "the callee is not a path, so nothing names what it calls",
            ));
        };
        let span = path.span();
        let segments: Vec<String> = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();

        // `Signal::Constant(v)` builds the enum, with whatever its payload says
        // about the enum's own parameters.
        if let Some((id, variant)) = self.registry.lookup_variant(self.module, &segments) {
            return Ok(self.variant_type(id, &variant, &call.args));
        }

        // `Self::new(..)`, `EntityId::from_bytes(..)`: the type named by
        // everything but the last segment, and an associated function on it.
        if segments.len() >= 2 {
            let name = segments.last().cloned().unwrap_or_default();
            if let Some(ty) = self.type_of_prefix(path) {
                // A MODULE resolves as a foreign type here — nothing declares
                // `serde_json`, and a prefix with no declaration is not
                // evidence that the last segment is an associated function. Ask
                // the value namespace first in that case: `serde_json::to_string`
                // is a free function returning `Result<String, Error>`, and
                // asking the type first found `ToString::to_string` through the
                // blanket impl and typed the call `String` — so the `.unwrap()`
                // written after it was dropped as an identity.
                let declared = ty
                    .id()
                    .is_some_and(|id| !id.is_foreign() && self.registry.def(id).is_some());
                if !declared {
                    if let Ok(Some(Def::Value(id))) =
                        self.registry.lookup(self.module, Ns::Value, &segments)
                    {
                        if let Some(ty) = self.registry.value(id).and_then(|v| v.ty.clone()) {
                            return Ok(ty);
                        }
                    }
                }
                if let Some(ret) = self.assoc_fn_return(&ty, &name) {
                    return Ok(ret);
                }
                // `bincode::serialize` is a function in a module, and a module
                // that nothing declares resolves as a foreign type — so a
                // prefix with no declaration is not evidence that the last
                // segment is an associated function. The value namespace is
                // asked before the refusal is written.
                let declared = ty
                    .id()
                    .is_some_and(|id| !id.is_foreign() && self.registry.def(id).is_some());
                if declared {
                    return Err(self.refuse(
                        span,
                        format!(
                            "no associated function `{}` on `{}`",
                            name,
                            self.registry.describe(&ty)
                        ),
                    ));
                }
            }
        }

        // A free function declared in reach.
        match self.registry.lookup(self.module, Ns::Value, &segments) {
            Ok(Some(Def::Value(id))) => match self.registry.value(id).and_then(|v| v.ty.clone()) {
                Some(ty) => Ok(ty),
                None => Err(self.refuse(
                    span,
                    format!("`{}` has no return type the engine could read", segments.join("::")),
                )),
            },
            Err(err) => Err(self.refuse(span, err.message)),
            _ => Err(self.refuse(
                span,
                format!("`{}` does not name a function here", segments.join("::")),
            )),
        }
    }

    /// The enum a variant belongs to, with its parameters bound from whatever
    /// the payload was given. A parameter the payload says nothing about is
    /// left standing as a parameter, which is the truth about it.
    pub(super) fn variant_type(
        &self,
        id: crate::ty::TypeId,
        variant: &str,
        args: &syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>,
    ) -> Ty {
        let params = self
            .registry
            .def(id)
            .map(|d| d.type_params.clone())
            .unwrap_or_default();
        let mut subst = Subst::new();
        if let Some(fields) = self.registry.variant_fields(id, variant) {
            for ((_, declared), arg) in fields.iter().zip(args) {
                if let Ok(actual) = self.resolve_expr(arg) {
                    let _ = unify(&params, declared, &actual, &mut subst);
                }
            }
        }
        Ty::Named {
            id,
            args: params
                .iter()
                .map(|p| subst.get(p).cloned().unwrap_or(Ty::Param(p.clone())))
                .collect(),
        }
    }

    /// The type everything but a path's last segment names.
    pub(super) fn type_of_prefix(&self, path: &syn::ExprPath) -> Option<Ty> {
        let mut prefix = path.path.clone();
        prefix.segments.pop();
        // `pop` leaves the trailing separator behind, which syn will print.
        while prefix.segments.trailing_punct() {
            let last = prefix.segments.pop()?.into_value();
            prefix.segments.push_value(last);
        }
        if prefix.segments.is_empty() {
            return None;
        }
        let ty = syn::Type::Path(syn::TypePath {
            qself: path.qself.clone(),
            path: prefix,
        });
        self.resolve_written_type(&ty).ok()
    }

    /// What an associated function on this type returns. Inherent impls first,
    /// then trait impls; two answers is no answer.
    pub(super) fn assoc_fn_return(&self, ty: &Ty, name: &str) -> Option<Ty> {
        let probe = self.probe();
        let mut inherent: Option<Ty> = None;
        let mut from_trait: Option<Ty> = None;
        let mut trait_count = 0;
        for id in self.registry.impls_for(ty) {
            let def = self.registry.impl_def(id);
            let Some(sig) = def.methods.get(name) else {
                continue;
            };
            if !sig.is_static() {
                continue;
            }
            let Some(subst) = def.match_self(ty) else {
                continue;
            };
            let ret = probe.normalize(&sig.ret.substitute(&subst));
            if def.is_inherent() {
                if inherent.is_some() {
                    return None;
                }
                inherent = Some(ret);
            } else {
                trait_count += 1;
                from_trait = Some(ret);
            }
        }
        inherent.or(if trait_count == 1 { from_trait } else { None })
    }

    /// What an associated function's arguments are declared to be, with the
    /// position it stands in used to close whatever the call leaves open.
    ///
    /// `Box::new(move |level| ..)` in a function returning `Box<dyn Fn(u32) ->
    /// bool>` is the case that matters: `new` declares `fn new(x: T) -> Box<T>`
    /// and says nothing about `T`; the return position does, and that is what
    /// gives the closure its parameter type (spec 4.5, 4.6).
    pub fn call_argument_types(
        &self,
        call: &syn::ExprCall,
        expected: Option<&Ty>,
    ) -> Vec<Option<Ty>> {
        self.call_argument_types_of(call, expected).unwrap_or_default()
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
        let name = path.path.segments.last()?.ident.to_string();
        // `Box::new(..)` names `Box` with no arguments, and the impl is written
        // for `Box<T>`; a bare `Box` does not resolve to a type at all. What
        // the position wants is the `Box<T>` this call is making, so where the
        // path names that constructor the expectation is the receiver the impl
        // is matched against.
        let owner = self
            .expectation_as_owner(path, expected)
            .or_else(|| self.type_of_prefix(path));
        let (sig, mut subst) = match owner {
            Some(owner) => self.static_method(&owner, &name)?,
            // A path with no type in front of it names a free function, whose
            // parameters are declared where the function is.
            None => (self.free_function_sig(path)?, Subst::new()),
        };
        // Whatever the position wants of the call binds the parameters the
        // signature left open.
        if let Some(want) = expected {
            let open = open_params(&sig.ret.substitute(&subst));
            let _ = unify(&open, &sig.ret.substitute(&subst), want.peel_refs(), &mut subst);
        }
        let probe = self.probe();
        Some(
            sig.params
                .iter()
                .map(|(_, ty)| {
                    let filled = probe.normalize(&ty.substitute(&subst));
                    (!expected::has_infer(&filled) && open_params(&filled).is_empty())
                        .then_some(filled)
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
    fn tuple_struct_argument_types(
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

    /// What a free function this path names declares.
    ///
    /// Resolved through the module that wrote the call, in the value namespace,
    /// so a `use` and a module-qualified path both arrive at the same
    /// declaration and a local of the same name never stands in for one.
    pub(super) fn free_function_sig(&self, path: &syn::ExprPath) -> Option<crate::registry::MethodSig> {
        let segments: Vec<String> = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        let crate::registry::Def::Value(id) = self
            .registry
            .lookup(self.module, crate::registry::Ns::Value, &segments)
            .ok()??
        else {
            return None;
        };
        self.registry.function_sig(id).cloned()
    }

    /// What each position of an enum variant's payload holds, where the path
    /// names a variant.
    ///
    /// `Err(e.into())` inside a function returning `Result<_, MutationError>`
    /// is the case that matters: the `.into()` has nothing else to read, and
    /// the variant is what says its payload is a `MutationError`. The enum's
    /// own parameters are closed by the position where the position names this
    /// enum, which is what makes `Ok`, `Err` and `Some` answer at all — their
    /// payload *is* a parameter.
    pub(super) fn variant_argument_types(
        &self,
        path: &syn::ExprPath,
        expected: Option<&Ty>,
    ) -> Option<Vec<Option<Ty>>> {
        let last = path.path.segments.last()?.ident.to_string();
        // Rust's prelude writes these with one segment, and the type they
        // belong to is named by the position rather than by the path.
        if path.path.segments.len() == 1 {
            let want = expected?.peel_refs();
            let Ty::Named { args, .. } = want else {
                return None;
            };
            return match last.as_str() {
                "Ok" if self.is_result(want) => Some(vec![args.first().cloned()]),
                "Err" if self.is_result(want) => Some(vec![args.get(1).cloned()]),
                "Some" if self.is_option(want) => Some(vec![args.first().cloned()]),
                _ => None,
            };
        }
        let segments: Vec<String> = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        let (id, variant) = self.registry.lookup_variant(self.module, &segments)?;
        let def = self.registry.def(id)?;
        let crate::registry::TypeKind::Enum { variants } = &def.kind else {
            return None;
        };
        let found = variants.iter().find(|v| v.name == variant)?;
        let mut subst = Subst::new();
        if let Some(Ty::Named { id: want, args }) = expected.map(|ty| ty.peel_refs()) {
            if *want == id {
                for (param, arg) in def.type_params.iter().zip(args) {
                    subst.insert(param.clone(), arg.clone());
                }
            }
        }
        Some(
            found
                .fields
                .iter()
                .map(|(_, ty)| {
                    let filled = ty.substitute(&subst);
                    (open_params(&filled).is_empty() && !expected::has_infer(&filled))
                        .then_some(filled)
                })
                .collect(),
        )
    }

    /// The type the position wants, where the path names its constructor.
    ///
    /// `Box::new` in a position wanting a `Box<dyn Fn(u32) -> bool>` names the
    /// `Box` the call is building; a bare `Box` does not resolve to a type on
    /// its own, and matching the impl needs the arguments the position knows.
    pub(super) fn expectation_as_owner(&self, path: &syn::ExprPath, expected: Option<&Ty>) -> Option<Ty> {
        let want = expected?.peel_refs();
        let id = want.id()?;
        let mut prefix = path.path.segments.iter().rev();
        prefix.next();
        let named = prefix.next()?.ident.to_string();
        (self.registry.name_of(id).rsplit("::").next() == Some(named.as_str()))
            .then(|| want.clone())
    }

    /// The one static method of that name reachable on this type, and what
    /// matching the impl bound its parameters to. Two answers is no answer.
    pub(super) fn static_method(&self, ty: &Ty, name: &str) -> Option<(crate::registry::MethodSig, Subst)> {
        let mut found: Option<(crate::registry::MethodSig, Subst)> = None;
        for id in self.registry.impls_for(ty) {
            let def = self.registry.impl_def(id);
            let Some(sig) = def.methods.get(name).filter(|sig| sig.is_static()) else {
                continue;
            };
            let Some(subst) = def.match_self(ty) else {
                continue;
            };
            if found.is_some() {
                return None;
            }
            found = Some((sig.clone(), subst));
        }
        found
    }

}

/// The expression a `(..)` or a `Group` was written around. Rust reads the
/// expression, not its punctuation: `(&v).into_iter()` is a call on a borrow
/// and `(0..n).contains(x)` is a call on a range, whatever the parentheses say.
pub(crate) fn unparenthesise(expr: &syn::Expr) -> &syn::Expr {
    match expr {
        syn::Expr::Paren(p) => unparenthesise(&p.expr),
        syn::Expr::Group(g) => unparenthesise(&g.expr),
        other => other,
    }
}
