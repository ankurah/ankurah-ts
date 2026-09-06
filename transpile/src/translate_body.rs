//! One function body, from `syn` to TypeScript.
//!
//! Split out of `main.rs`, which was over the 600-line rule and grew again when
//! the `#[cfg(test)] mod testing;` squash was undone (R10: the ratchet is met by
//! splitting, never by joining lines). What is here is the body walk and the two
//! questions it asks on the way — whether a written type is an alias the port
//! resolves, and what a type resolves to when nothing is reported.

use crate::{body, diag, extract, infer, name_map, registry, ty, types};

/// Translate a single function's body_ast → body_ts with type-aware context.
#[allow(clippy::too_many_arguments)]
pub(crate) fn translate_fn_body(
    func: &mut types::FnInfo,
    self_type: &str,
    // The identifier Rust's `self` is emitted as: `this` for a method on an
    // emitted class, and the function's first parameter for a method whose
    // impl has no class of its own.
    self_name: &'static str,
    self_ty: Option<ty::Ty>,
    impl_params: &[String],
    impl_bounds: &[(String, ty::TraitRef)],
    registry: &registry::TypeRegistry,
    module: registry::ModuleId,
    inline_module_names: &[String],
    consts: &[(String, ty::Ty)],
    sink: &diag::DiagSink,
) {
    if let Some(ref block) = func.body_ast {
        let mut params = impl_params.to_vec();
        params.extend(func.type_params.iter().cloned());

        let mut tc = infer::TypeContext::new(registry, module, self_ty.clone(), params, sink);
        // The bounds in scope: the impl block's, plus this function's own.
        let mut bounds = impl_bounds.to_vec();
        {
            let env = registry::TypeEnv::new(registry, module, sink)
                .with_params(&tc.params)
                .with_self(self_ty.as_ref());
            bounds.extend(registry::method::param_bounds_of(&registry::resolve_bounds(
                &func.syn_generics,
                &env,
                sink,
            )));
        }
        tc.param_bounds = bounds;
        for (name, ty) in consts {
            tc.bind(name, ty.clone());
        }

        let typed_params: Vec<(String, ty::Ty)> = func
            .params
            .iter()
            .filter_map(|p| {
                let syn_ty = p.rust_ty.as_ref()?;
                match tc.resolve_written_type(syn_ty) {
                    Ok(ty) => Some((p.name.clone(), ty)),
                    Err(diag) => {
                        sink.push(diag);
                        None
                    }
                }
            })
            .collect();
        tc.push_fn(typed_params);

        // Every parameter and the return type, written from the type the
        // engine resolved rather than from the syntax.
        //
        // The syntactic mapping cannot see what a name means: `Self` came out
        // as the word `Self`, `Self::Target` as the bare associated name, and a
        // crate type sharing a leaf with a std one as the std one's spelling.
        // `map_ty` reproduces that mapping case for case where the two agree,
        // so a signature only moves where the syntax was wrong. A type the
        // engine could not name keeps what it had, which is where it stood
        // before.
        // A type parameter whose only bound is a callable one is written as the
        // callable itself, and the resolution here would put the bare parameter
        // name back — which names nothing, because the generics list no longer
        // declares it.
        let arguments: Vec<syn::Type> =
            func.params.iter().filter_map(|p| p.rust_ty.clone()).collect();
        let callables = extract::callable_only_params_of(
            &func.syn_generics,
            &arguments,
            func.rust_return.as_ref(),
        );
        for param in func.params.iter_mut() {
            let Some(written) = param.rust_ty.as_ref() else {
                continue;
            };
            // The reference is peeled, because emission erases it: `f: &mut F`
            // and `f: F` are one TypeScript parameter, and the rule that writes
            // the callable in place of the parameter name has to reach both.
            if let syn::Type::Path(path) = extract::peel_written_refs(written) {
                if let Some(name) = path.path.get_ident().map(|i| i.to_string()) {
                    if let Some(spelling) = callables.get(&name) {
                        param.ty = spelling.clone();
                        continue;
                    }
                }
            }
            if names_an_alias(registry, module, written) {
                continue;
            }
            if let Ok(resolved) = quiet_type(&tc, written) {
                param.ty = name_map::map_ty(registry, &resolved);
            }
        }
        if let Some(written) = func.rust_return.as_ref() {
            if !names_an_alias(registry, module, written) {
                if let Ok(resolved) = quiet_type(&tc, written) {
                    func.return_type = name_map::map_ty(registry, &resolved);
                }
            }
        }

        // What this function returns, so that `?` can say whether the error it
        // hands on needs a `From` conversion Rust would have called.
        let returns = func
            .rust_return
            .as_ref()
            .and_then(|written| tc.resolve_written_type(written).ok());

        // Rust drops a by-value parameter at the end of the function body, so
        // the body's block owns it exactly as it owns its own locals. A `&self`
        // or `&T` parameter is a borrow and owns nothing.
        let mut owned_params: Vec<(String, ty::Ty)> = func
            .params
            .iter()
            .filter(|p| !matches!(p.rust_ty, Some(syn::Type::Reference(_))))
            .filter_map(|p| {
                let syn_ty = p.rust_ty.as_ref()?;
                Some((p.name.clone(), tc.resolve_written_type(syn_ty).ok()?))
            })
            .collect();
        // `fn into_inner(self)` takes the receiver by value, so the body owns it
        // like any other by-value parameter: the caller stops owning it at the
        // call, and if the body does not hand it on, the body releases it.
        // Leaving it out made every self-taking method a leak.
        if func.self_kind == Some(types::SelfKind::Value) {
            if let Some(ty) = self_ty.clone() {
                owned_params.insert(0, (self_name.to_string(), ty));
            }
        }

        let mut translator = body::BodyTranslator::with_context(self_type, tc);
        // C1: a `&mut T` parameter whose `T` the port writes as a JavaScript
        // VALUE is a `BorrowMut<T>`, and the body reads and writes it through
        // `.value`. Without the cell the callee's writes went nowhere.
        let cell_params: Vec<String> = func
            .params
            .iter()
            .filter(|p| body::cells::is_boxed_mut(p))
            .map(|p| p.name.clone())
            .collect();
        *translator.boxed.borrow_mut() = cell_params.clone();
        *translator.cell_params.borrow_mut() = cell_params;
        translator.self_name = self_name;
        translator.inline_module_names = inline_module_names.to_vec();
        translator.fn_return = returns;
        translator.owns_self = func.self_kind == Some(types::SelfKind::Value);
        // A `fmt` taking a `Formatter` is a formatter body: its `write!` calls
        // compose one string, and the `Ok(())` it ends with is that string.
        translator.formatter = func.name == "fmt"
            && !body::writes_once_at_the_tail(block)
            && func.params.iter().any(|p| {
                p.rust_ty.as_ref().is_some_and(|ty| {
                    let written = quote::ToTokens::to_token_stream(ty).to_string();
                    written.contains("Formatter")
                })
            });
        // I1: the lowering records whether it refused a shape, rather than
        // the emitter searching the rendered body for `unsupported(`.
        let holes_before = body::holes_written();
        func.body_ts = Some(translator.translate_fn_block(block, &owned_params));
        func.body_has_hole = body::holes_written() > holes_before;
        translator.pop_scope();
        // Fallbacks taken on translation paths that carry no sink of their own.
        diag::pending::drain(sink);
    }
    if func.body_ts.is_some() {
        func.body_ast = None;
    }
}


/// Is this TypeScript spelling a value JavaScript copies?
pub(crate) fn is_value_spelling(ty: &str) -> bool {
    let bare = ty.strip_suffix(" | null").unwrap_or(ty);
    matches!(bare, "string" | "number" | "boolean" | "bigint")
}

/// Does this written type name a type alias?
///
/// A resolved type has no memory of the alias it was written as, so writing the
/// signature from it turns `Listener` into the `Arc<dyn Fn(T)>` the alias
/// stands for. The port emits the alias, and the alias is what the source said,
/// so the syntactic spelling stays where one is named.
pub(crate) fn names_an_alias(
    registry: &registry::TypeRegistry,
    module: registry::ModuleId,
    written: &syn::Type,
) -> bool {
    match written {
        syn::Type::Path(path) => {
            let segments: Vec<String> =
                path.path.segments.iter().map(|s| s.ident.to_string()).collect();
            if matches!(
                registry.lookup_type(module, &segments),
                Ok(Some(registry::Def::Alias(_)))
            ) {
                return true;
            }
            // An alias UNDER a wrapper is still an alias the port emits:
            // `Arc<Listener>` and `Vec<Listener>` name one as surely as a bare
            // `Listener` does, and reading only the outermost name expanded
            // them into the `Arc<dyn Fn(T)>` the alias stands for.
            path.path
                .segments
                .last()
                .into_iter()
                .filter_map(|segment| match &segment.arguments {
                    syn::PathArguments::AngleBracketed(args) => Some(args),
                    _ => None,
                })
                .flat_map(|args| args.args.iter())
                .any(|arg| match arg {
                    syn::GenericArgument::Type(ty) => names_an_alias(registry, module, ty),
                    _ => false,
                })
        }
        // A reference is erased in emission, so what it points at decides.
        syn::Type::Reference(r) => names_an_alias(registry, module, &r.elem),
        syn::Type::Paren(p) => names_an_alias(registry, module, &p.elem),
        syn::Type::Group(g) => names_an_alias(registry, module, &g.elem),
        syn::Type::Slice(s) => names_an_alias(registry, module, &s.elem),
        syn::Type::Array(a) => names_an_alias(registry, module, &a.elem),
        syn::Type::Tuple(t) => t.elems.iter().any(|e| names_an_alias(registry, module, e)),
        _ => false,
    }
}

/// A written type resolved and read through the impl table, with no diagnostic
/// filed for it.
///
/// The body translation asks the same questions and reports what it could not
/// answer; asking again here to write the signature would count each gap twice.
pub(crate) fn quiet_type(tc: &infer::TypeContext<'_>, written: &syn::Type) -> Result<ty::Ty, diag::Diag> {
    let mark = tc.sink.mark();
    let resolved = tc.resolve_written_type(written);
    tc.sink.rewind(mark);
    Ok(tc.probe().normalize(&resolved?))
}

