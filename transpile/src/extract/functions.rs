//! A `fn` item, read for everything the emitters need from it — and, for one
//! at MODULE level, the BINDING its emitted name is.
//!
//! A module-level function is a binding, and JavaScript refuses a reserved word
//! there: `pub fn with` is `with_`. That answer is allocated once, here, and
//! read by the declaration, by every call, by the cross-file map that says
//! which module a name comes from, and by the import and re-export lists.
//! Allocated in one place and re-derived in the others, the declaration said
//! `with_` and the import said `with`, which matched nothing — so the import
//! was not written at all and the call named an undeclared binding, silently
//! (FF3/GG2).

use super::generics::{extract_generics_without, type_param_names};
use super::{is_test_fn, receiver_kind};
use crate::extract::generics::callable_only_params;
#[cfg(test)]
#[path = "binding_tests.rs"]
mod binding_tests;

use crate::name_map;
use syn::{FnArg, ReturnType};
use crate::types::*;

pub(super) fn extract_fn_with_body(
    sig: &syn::Signature,
    is_pub: bool,
    vis: VisInfo,
    attrs: &[syn::Attribute],
    body: Option<&syn::Block>,
    features: Option<&crate::cfg::CfgFeatures>,
) -> FnInfo {
    let mut info = extract_fn_vis(sig, is_pub, vis, attrs);
    if let Some(block) = body {
        let mut block = block.clone();
        // A `#[cfg]` inside a body decides whether the statement is in this
        // build, exactly as it does for an item. Pruning here means nothing
        // downstream has to ask again.
        if let Some(features) = features {
            crate::cfg::prune_block(&mut block, features);
        }
        info.body_ast = Some(block);
    }
    info
}

/// Extract function with body, recording the self type for later translation.
/// The self_type is stored on the ImplInfo, not the FnInfo — the translation phase
/// uses ImplInfo.target_type to create the ImplScope.
pub(super) fn extract_fn_with_body_and_self(
    sig: &syn::Signature,
    is_pub: bool,
    attrs: &[syn::Attribute],
    body: Option<&syn::Block>,
    _self_type: &str,
    features: Option<&crate::cfg::CfgFeatures>,
) -> FnInfo {
    // self_type is no longer used during extraction — it's resolved from ImplInfo during Phase 3
    let vis = if is_pub { VisInfo::Public } else { VisInfo::Private };
    extract_fn_with_body(sig, is_pub, vis, attrs, body, features)
}

pub(super) fn extract_fn(sig: &syn::Signature, is_pub: bool, attrs: &[syn::Attribute]) -> FnInfo {
    extract_fn_vis(sig, is_pub, if is_pub { VisInfo::Public } else { VisInfo::Private }, attrs)
}

pub(super) fn extract_fn_vis(sig: &syn::Signature, is_pub: bool, vis: VisInfo, attrs: &[syn::Attribute]) -> FnInfo {
    let rust_name = sig.ident.to_string();
    let ts_name = name_map::map_fn_name(&rust_name);
    let is_async = sig.asyncness.is_some();

    let mut is_static = true;
    let mut self_kind = None;
    let mut self_receiver = None;
    let params: Vec<ParamInfo> = sig.inputs.iter().filter_map(|arg| {
        match arg {
            FnArg::Receiver(r) => {
                is_static = false;
                self_kind = Some(receiver_kind(r));
                if self_kind == Some(SelfKind::Arbitrary) {
                    self_receiver = Some((*r.ty).clone());
                }
                None
            }
            FnArg::Typed(pat) => {
                let name = if let syn::Pat::Ident(ident) = &*pat.pat {
                    name_map::escape_reserved(&name_map::to_camel_case(&ident.ident.to_string()))
                } else {
                    "arg".to_string()
                };
                Some(ParamInfo {
                    name,
                    ty: name_map::map_type(&pat.ty),
                    rust_ty: Some((*pat.ty).clone()),
                })
            }
        }
    }).collect();

    let (return_type, rust_return) = match &sig.output {
        ReturnType::Default => ("void".to_string(), None),
        ReturnType::Type(_, ty) => (name_map::map_type(ty), Some((**ty).clone())),
    };

    // A type parameter whose only bound is a CALLABLE one, and whose only use is
    // a parameter's type, is written as the callable itself. TypeScript infers
    // nothing through a type parameter constrained by a union, so
    // `<F extends Invocable<[number], number>>(f: F)` made `invoke(f, n)`
    // answer `unknown` and every use of that answer a type error; the parameter
    // spelled `f: Invocable<[number], number>` says the same thing and infers.
    let callables = callable_only_params(sig);

    let params: Vec<ParamInfo> = params
        .into_iter()
        .map(|mut p| {
            if let Some(spelling) = callables.get(&p.ty) {
                p.ty = spelling.clone();
            }
            p
        })
        .collect();

    FnInfo {
        name: rust_name,
        ts_name,
        is_pub,
        vis,
        is_async,
        is_static,
        self_kind,
        self_receiver,
        has_default_body: false,
        params,
        return_type,
        rust_return,
        generics: extract_generics_without(&sig.generics, &callables),
        type_params: type_param_names(&sig.generics),
        syn_generics: sig.generics.clone(),
        is_test: is_test_fn(attrs),
        body_ast: None,
        body_ts: None,
        body_has_hole: false,
    }
}

/// The BINDING each module-level function is emitted under, allocated over the
/// whole module at once so that a collision is seen rather than written.
///
/// GG2: `escape_reserved` is a suffix, and a suffix cannot be injective — valid
/// Rust declaring both `fn r#in()` and `fn in_()` gives two
/// `export function in_()`, which is `TS2323` and `TS2393` and a body calling
/// either one of them reaching whichever came second. The port has no second
/// answer to give (a renamed binding would have to be renamed at every call
/// site too, in a later phase), so the collision is REPORTED at the file, with
/// both Rust names, rather than emitted in silence.
pub(super) fn report_binding_collisions(file: &RustFile) {
    for (_, inline) in &file.inline_modules {
        report_binding_collisions(inline);
    }
    let mut taken: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for f in &file.functions {
        if let Some(first) = taken.insert(f.ts_name.clone(), f.name.clone()) {
            crate::diag::pending::park_at(
                0,
                0,
                format!(
                    "`{}` and `{}` are both written `{}` at module level, and TypeScript \
                     takes one binding per name: the second declaration wins and \
                     every call to either of them reaches it",
                    first, f.name, f.ts_name
                ),
            );
        }
    }
}
