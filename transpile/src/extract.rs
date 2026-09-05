//! Rust source extraction via syn
//!
//! Parses .rs files and extracts structs, enums, traits, functions,
//! impl blocks, use statements into the intermediate data structures.

use anyhow::{Context, Result};
use std::path::Path;
use quote::ToTokens;
use syn::{self, Visibility, FnArg, ReturnType, Fields};

use crate::name_map;
use crate::types::*;

/// What the build says about the file being read: which features are on, and
/// which of its items the port leaves out one at a time.
///
/// Both are questions only `transpile.toml` can answer, and both decide whether
/// an item exists at all — so they are asked here, at extraction, rather than
/// later where an absent item looks like a bug.
#[derive(Default, Clone, Copy)]
pub struct ExtractCfg<'a> {
    pub features: Option<&'a crate::cfg::CfgFeatures>,
    /// The `[[excluded_items]]` entries naming items in THIS file.
    pub excluded: &'a [&'a crate::config::ExcludedItem],
}

impl<'a> ExtractCfg<'a> {
    pub fn features(features: Option<&'a crate::cfg::CfgFeatures>) -> Self {
        ExtractCfg {
            features,
            excluded: &[],
        }
    }
}

/// Extract all items from a Rust source file.
/// If features is provided, #[cfg(...)] expressions are evaluated against it.
/// Otherwise, only wasm/uniffi are skipped (legacy behavior).
pub fn extract(path: &Path) -> Result<RustFile> {
    extract_with_cfg(path, ExtractCfg::default())
}

pub fn extract_with_cfg(path: &Path, cfg: ExtractCfg) -> Result<RustFile> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    extract_source(&path.display().to_string(), &content, cfg)
}

/// Extract from source text already in hand. Batch reads from disk; the engine's
/// unit tests hand it a few lines of Rust directly.
pub fn extract_source(path: &str, content: &str, cfg: ExtractCfg) -> Result<RustFile> {
    let syntax = syn::parse_file(content)
        .with_context(|| format!("Failed to parse {}", path))?;

    let mut file = RustFile::empty(path.to_string());
    extract_items(&syntax.items, cfg, &mut file);
    Ok(file)
}

thread_local! {
    /// Which `[[excluded_items]]` selectors matched something while reading the
    /// current file. An entry that never matches is a config that has gone
    /// stale against the corpus, and the caller reports it.
    static EXCLUSIONS_HIT: std::cell::RefCell<std::collections::BTreeSet<String>> =
        std::cell::RefCell::new(std::collections::BTreeSet::new());
}

/// Take the selectors that matched since the last call.
pub fn take_exclusions_hit() -> std::collections::BTreeSet<String> {
    EXCLUSIONS_HIT.with(|h| std::mem::take(&mut *h.borrow_mut()))
}

fn note_exclusion(item: &crate::config::ExcludedItem) {
    EXCLUSIONS_HIT.with(|h| h.borrow_mut().insert(item.written.clone()));
}

/// Is this item named by an `[[excluded_items]]` entry? Records the hit.
fn is_excluded_item(item: &syn::Item, cfg: ExtractCfg) -> bool {
    for entry in cfg.excluded {
        if entry.selector.matches(item) {
            note_exclusion(entry);
            return true;
        }
    }
    false
}

/// Report a `#[cfg]` no configuration decides. The item is kept: dropping it
/// would take every type it declares with it, and the report would be a crowd of
/// unrelated "no declaration for" lines somewhere else.
fn report_undecided_cfg(span: proc_macro2::Span, text: &str) {
    crate::diag::pending::park(
        span,
        format!(
            "`#[{}]` has a predicate nothing in transpile.toml's [cfg] or [features] decides; \
             the item is kept",
            text
        ),
    );
}

/// Read one module's items into `file`. A file's top level and an inline
/// `mod x { .. }` are the same thing to Rust, so they are read by the same walk.
fn extract_items(items: &[syn::Item], cfg: ExtractCfg, file: &mut RustFile) {
    let features = cfg.features;
    for item in items {
        if is_excluded_item(item, cfg) {
            continue;
        }
        match item {
            syn::Item::Struct(s) => {
                if is_skipped_cfg_with(&s.attrs, features) { continue; }
                file.structs.push(extract_struct(s));
            }
            syn::Item::Enum(e) => {
                if is_skipped_cfg_with(&e.attrs, features) { continue; }
                file.enums.push(extract_enum(e));
            }
            syn::Item::Trait(t) => {
                if is_skipped_cfg_with(&t.attrs, features) { continue; }
                file.traits.push(extract_trait(t, cfg));
            }
            syn::Item::Fn(f) => {
                if is_skipped_cfg_with(&f.attrs, features) { continue; }
                file.functions.push(extract_fn_with_body(&f.sig, is_public(&f.vis), visibility(&f.vis), &f.attrs, Some(&f.block)));
            }
            syn::Item::Impl(i) => {
                if is_skipped_cfg_with(&i.attrs, features) { continue; }
                file.impls.push(extract_impl(i, cfg));
            }
            syn::Item::Use(u) => {
                file.uses.push(extract_use(u));
            }
            syn::Item::Type(t) => {
                if is_skipped_cfg_with(&t.attrs, features) { continue; }
                file.type_aliases.push(TypeAliasInfo {
                    name: t.ident.to_string(),
                    ty: name_map::map_type(&t.ty),
                    rust_ty: (*t.ty).clone(),
                    is_pub: is_public(&t.vis),
                    vis: visibility(&t.vis),
                    type_params: type_param_names(&t.generics),
                    param_defaults: type_param_defaults(&t.generics),
                });
            }
            syn::Item::Const(c) => {
                if is_skipped_cfg_with(&c.attrs, features) { continue; }
                file.consts.push(ConstInfo {
                    name: c.ident.to_string(),
                    ty: name_map::map_type(&c.ty),
                    rust_ty: Some((*c.ty).clone()),
                    is_pub: is_public(&c.vis),
                    vis: visibility(&c.vis),
                });
            }
            syn::Item::Mod(m) => {
                // Check for #[cfg(test)] mod tests { ... }
                if is_test_module(&m.attrs) {
                    if let Some((_, items)) = &m.content {
                        for item in items {
                            if let syn::Item::Fn(f) = item {
                                let extracted = extract_fn_with_body(&f.sig, true, VisInfo::Private, &f.attrs, Some(&f.block));
                                if is_test_fn(&f.attrs) {
                                    file.test_functions.push(extracted);
                                } else {
                                    // A helper the tests call. It is part of
                                    // the suite, not of the module.
                                    file.test_helpers.push(extracted);
                                }
                            }
                        }
                    }
                }
                // Extract inline modules as separate RustFile entries.
                // These become sibling .ts files (e.g., context/stack.ts).
                //
                // An inline module reads exactly like a file, through the same
                // walk: an `impl` or a `trait` written inside one is as real as
                // one written at the top of a file, and reading only some of the
                // item kinds dropped those declarations on the floor. The std
                // surface's `pub mod` blocks — tokio's `oneshot` and `mpsc`,
                // serde's `ser` and `de` — are nothing but impls and traits.
                else if !is_skipped_cfg_with(&m.attrs, features) {
                    if m.content.is_none() {
                        // `mod x;` — the file beside this one. Rust needs the
                        // declaration for the module to exist at all, and the
                        // emitted index needs it to know what to re-export.
                        file.mod_decls.push((m.ident.to_string(), visibility(&m.vis)));
                    }
                    if let Some((_, items)) = &m.content {
                        let mod_name = m.ident.to_string();
                        let mut sub_file = RustFile::empty(String::new());
                        sub_file.vis = visibility(&m.vis);
                        extract_items(items, cfg, &mut sub_file);
                        file.inline_modules.push((mod_name, sub_file));
                    }
                }
            }
            syn::Item::Macro(mac) => {
                // Handle thread_local! { static NAME: TYPE = INIT; }
                let macro_name = mac.mac.path.segments.last()
                    .map(|s| s.ident.to_string()).unwrap_or_default();
                if macro_name == "thread_local" {
                    if let Some((decl, name, ty, rust_ty)) = extract_thread_local(&mac.mac) {
                        file.module_decls.push(decl);
                        file.consts.push(ConstInfo { name, ty, rust_ty, is_pub: false, vis: VisInfo::Private });
                    }
                }
            }
            _ => {}
        }
    }
}

fn is_public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

/// Visibility as written. `pub(in path)` keeps its position so the registry can
/// report it rather than widen it in silence.
fn visibility(vis: &Visibility) -> VisInfo {
    match vis {
        Visibility::Public(_) => VisInfo::Public,
        Visibility::Restricted(r) => {
            let first = r.path.segments.first().map(|s| s.ident.to_string());
            match first.as_deref() {
                Some("self") => VisInfo::Private,
                Some("super") => VisInfo::Super,
                Some("crate") if r.path.segments.len() == 1 => VisInfo::Crate,
                _ => {
                    let start = syn::spanned::Spanned::span(&r.path).start();
                    VisInfo::InPath { line: start.line, col: start.column + 1 }
                }
            }
        }
        Visibility::Inherited => VisInfo::Private,
    }
}

fn is_skipped_cfg(attrs: &[syn::Attribute]) -> bool {
    is_skipped_cfg_with(attrs, None)
}

fn is_skipped_cfg_with(attrs: &[syn::Attribute], features: Option<&crate::cfg::CfgFeatures>) -> bool {
    if let Some(features) = features {
        return match crate::cfg::gate(attrs, features) {
            crate::cfg::Gate::Keep => false,
            crate::cfg::Gate::Skip => true,
            crate::cfg::Gate::Undecided(text) => {
                let span = attrs
                    .first()
                    .map(syn::spanned::Spanned::span)
                    .unwrap_or_else(proc_macro2::Span::call_site);
                report_undecided_cfg(span, &text);
                false
            }
        };
    }
    // Legacy fallback: string matching for wasm/uniffi
    for attr in attrs {
        if attr.path().is_ident("cfg") {
            let tokens = attr.meta.to_token_stream().to_string();
            if tokens.contains("wasm") || tokens.contains("uniffi") {
                return true;
            }
        }
    }
    false
}

/// The format string of a `#[error("..")]`, which is what thiserror's derive
/// renders that variant as.
///
/// Only the plain string form is read. `#[error(transparent)]` and an `#[error]`
/// carrying its own arguments are different lowerings, and returning `None` for
/// them leaves the caller to say so at the variant rather than rendering the
/// wrong text.
fn error_attribute(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("error") {
            continue;
        }
        let list = attr.meta.require_list().ok()?;
        return syn::parse2::<syn::LitStr>(list.tokens.clone())
            .ok()
            .map(|lit| lit.value());
    }
    None
}

fn extract_derives(attrs: &[syn::Attribute]) -> Vec<String> {
    let mut derives = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("derive") {
            if let Ok(meta_list) = attr.meta.require_list() {
                let tokens = meta_list.tokens.to_string();
                for part in tokens.split(',') {
                    derives.push(part.trim().to_string());
                }
            }
        }
    }
    derives
}

fn is_test_module(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("cfg") {
            let tokens = attr.meta.to_token_stream().to_string();
            tokens.contains("test")
        } else {
            false
        }
    })
}

fn is_test_fn(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("test") ||
        a.meta.to_token_stream().to_string().contains("tokio :: test")
    })
}

fn extract_struct(s: &syn::ItemStruct) -> StructInfo {
    StructInfo {
        name: s.ident.to_string(),
        is_pub: is_public(&s.vis),
        vis: visibility(&s.vis),
        fields: extract_fields(&s.fields),
        generics: extract_generics(&s.generics),
        type_params: type_param_names(&s.generics),
        param_defaults: type_param_defaults(&s.generics),
        derives: extract_derives(&s.attrs),
        span: s.ident.span(),
    }
}

fn extract_enum(e: &syn::ItemEnum) -> EnumInfo {
    let variants = e.variants.iter().map(|v| {
        let is_serde_other = v.attrs.iter().any(|a| {
            if let syn::Meta::List(meta) = &a.meta {
                if meta.path.is_ident("serde") {
                    return meta.tokens.to_string().contains("other");
                }
            }
            false
        });
        // `#[serde(with = "..")]` sits on the VARIANT, not on the field
        // inside it: `#[serde(with = "json_as_bytes")] Json(serde_json::Value)`.
        // The codec asks the field, so the variant's answer stands in for it.
        let mut fields = extract_fields(&v.fields);
        if let Some(module) = serde_with_attr(&v.attrs) {
            for field in &mut fields {
                field.serde_with.get_or_insert(module.clone());
            }
        }
        VariantInfo {
            name: v.ident.to_string(),
            fields,
            is_serde_other,
            error_format: error_attribute(&v.attrs),
            span: v.ident.span(),
        }
    }).collect();

    EnumInfo {
        name: e.ident.to_string(),
        is_pub: is_public(&e.vis),
        vis: visibility(&e.vis),
        variants,
        generics: extract_generics(&e.generics),
        type_params: type_param_names(&e.generics),
        param_defaults: type_param_defaults(&e.generics),
        derives: extract_derives(&e.attrs),
        span: e.ident.span(),
    }
}

fn extract_trait(t: &syn::ItemTrait, cfg: ExtractCfg) -> TraitInfo {
    let trait_name = t.ident.to_string();
    let excluded_assoc = |name: &str| {
        for entry in cfg.excluded {
            if entry.selector.matches_assoc_type(&trait_name, name) {
                note_exclusion(entry);
                return true;
            }
        }
        false
    };
    let mut has_default_impls = false;
    let methods = t.items.iter().filter_map(|item| {
        if let syn::TraitItem::Fn(method) = item {
            if method.default.is_some() {
                has_default_impls = true;
            }
            let mut info = extract_fn(&method.sig, true, &method.attrs);
            info.has_default_body = method.default.is_some();
            // A default body is ordinary code that every implementor inherits.
            // Keeping only the flag meant emission wrote a `throw` in its place
            // and each implementor that omitted the method lost it.
            info.body_ast = method.default.clone();
            Some(info)
        } else {
            None
        }
    }).collect();

    let supertraits = t.supertraits.iter().filter_map(|b| match b {
        syn::TypeParamBound::Trait(t) => Some(t.clone()),
        _ => None,
    }).collect();
    let assoc_types = t.items.iter().filter_map(|item| match item {
        syn::TraitItem::Type(ty) if !excluded_assoc(&ty.ident.to_string()) => Some(ty.ident.to_string()),
        _ => None,
    }).collect();

    TraitInfo {
        name: trait_name,
        is_pub: is_public(&t.vis),
        vis: visibility(&t.vis),
        is_auto: t.auto_token.is_some(),
        methods,
        has_default_impls,
        generics: extract_generics(&t.generics),
        type_params: type_param_names(&t.generics),
        supertraits,
        assoc_types,
    }
}

/// Extract function with body — stores the raw syn::Block for deferred translation.
/// Body translation happens in Phase 3 (with full type context), not during extraction.
/// Extract thread_local! { static NAME: TYPE = INIT; } → const NAME = new ThreadLocal<TYPE>(INIT);
/// Returns (decl_string, const_name, const_type) for structured tracking
fn extract_thread_local(mac: &syn::Macro) -> Option<(String, String, String, Option<syn::Type>)> {
    // Parse the macro body as a static item
    let tokens = mac.tokens.clone();
    // Try to parse as: static NAME: TYPE = EXPR;
    if let Ok(item) = syn::parse2::<syn::ItemStatic>(tokens) {
        let name = item.ident.to_string();
        let ty = name_map::map_type(&item.ty);
        let init = crate::body::translate_expr(&item.expr);
        let full_type = format!("ThreadLocal<{}>", ty);
        let decl = format!("const {} = new ThreadLocal<{}>({});", name, ty, init);
        // What `thread_local!` declares is a `std::thread::LocalKey<T>`; the
        // port calls it `ThreadLocal`, and the system declaration carries both
        // names. The engine is told the Rust one.
        let rust_source = format!("std::thread::LocalKey<{}>", item.ty.to_token_stream());
        let rust_ty = syn::parse_str::<syn::Type>(&rust_source).ok();
        Some((decl, name, full_type, rust_ty))
    } else {
        None
    }
}

fn extract_fn_with_body(sig: &syn::Signature, is_pub: bool, vis: VisInfo, attrs: &[syn::Attribute], body: Option<&syn::Block>) -> FnInfo {
    let mut info = extract_fn_vis(sig, is_pub, vis, attrs);
    if let Some(block) = body {
        info.body_ast = Some(block.clone());
    }
    info
}

/// Extract function with body, recording the self type for later translation.
/// The self_type is stored on the ImplInfo, not the FnInfo — the translation phase
/// uses ImplInfo.target_type to create the ImplScope.
fn extract_fn_with_body_and_self(sig: &syn::Signature, is_pub: bool, attrs: &[syn::Attribute], body: Option<&syn::Block>, _self_type: &str) -> FnInfo {
    // self_type is no longer used during extraction — it's resolved from ImplInfo during Phase 3
    let vis = if is_pub { VisInfo::Public } else { VisInfo::Private };
    extract_fn_with_body(sig, is_pub, vis, attrs, body)
}

fn extract_fn(sig: &syn::Signature, is_pub: bool, attrs: &[syn::Attribute]) -> FnInfo {
    extract_fn_vis(sig, is_pub, if is_pub { VisInfo::Public } else { VisInfo::Private }, attrs)
}

fn extract_fn_vis(sig: &syn::Signature, is_pub: bool, vis: VisInfo, attrs: &[syn::Attribute]) -> FnInfo {
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
                    is_self: false,
                    is_mut_self: false,
                })
            }
        }
    }).collect();

    let (return_type, rust_return) = match &sig.output {
        ReturnType::Default => ("void".to_string(), None),
        ReturnType::Type(_, ty) => (name_map::map_type(ty), Some((**ty).clone())),
    };

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
        generics: extract_generics(&sig.generics),
        type_params: type_param_names(&sig.generics),
        syn_generics: sig.generics.clone(),
        is_test: is_test_fn(attrs),
        body_ast: None,
        body_ts: None,
    }
}

fn extract_impl(i: &syn::ItemImpl, cfg: ExtractCfg) -> ImplInfo {
    let self_type_name = if let syn::Type::Path(p) = &*i.self_ty {
        p.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default()
    } else {
        String::new()
    };

    // For TryInto<Target>/Into<Target> where self_ty is a stdlib type,
    // flip the target to the trait's type arg (the actual destination type)
    let target_type = if let Some((_, path, _)) = &i.trait_ {
        let trait_name = path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
        if matches!(trait_name.as_str(), "TryInto" | "Into") {
            // Extract the target type from the trait's generic arg
            if let Some(seg) = path.segments.last() {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(syn::Type::Path(target_path))) = args.args.first() {
                        if let Some(target_seg) = target_path.path.segments.last() {
                            target_seg.ident.to_string()
                        } else {
                            self_type_name.clone()
                        }
                    } else {
                        self_type_name.clone()
                    }
                } else {
                    self_type_name.clone()
                }
            } else {
                self_type_name.clone()
            }
        } else {
            self_type_name.clone()
        }
    } else {
        self_type_name
    };

    let methods = i.items.iter().filter_map(|item| {
        if let syn::ImplItem::Fn(method) = item {
            if is_skipped_cfg(&method.attrs) { return None; }
            for entry in cfg.excluded {
                if entry.selector.matches_method(&i.self_ty, method) {
                    note_exclusion(entry);
                    return None;
                }
            }
            Some(extract_fn_with_body_and_self(
                &method.sig, is_public(&method.vis), &method.attrs,
                Some(&method.block), &target_type))
        } else {
            None
        }
    }).collect();

    let assoc_types = i.items.iter().filter_map(|item| match item {
        syn::ImplItem::Type(ty) => Some((ty.ident.to_string(), ty.ty.clone())),
        _ => None,
    }).collect();

    ImplInfo {
        target_type,
        self_ty: Some((*i.self_ty).clone()),
        type_params: type_param_names(&i.generics),
        trait_path: i.trait_.as_ref().map(|(_, path, _)| path.clone()),
        generics: i.generics.clone(),
        assoc_types,
        methods,
    }
}

/// How a method's `self` parameter is written.
///
/// `self`, `&self` and `&mut self` are the three the engine models. `self: T`
/// for any other `T` — `Arc<Self>`, `Pin<&mut Self>` — is its own kind: reading
/// it as by-value would say the method sits on `Self` when it sits on the
/// wrapper, and put it on the wrong step of the deref chain.
fn receiver_kind(r: &syn::Receiver) -> SelfKind {
    if r.colon_token.is_some() {
        return SelfKind::Arbitrary;
    }
    match &r.reference {
        Some((_, _)) if r.mutability.is_some() => SelfKind::RefMut,
        Some((_, _)) => SelfKind::Ref,
        None => SelfKind::Value,
    }
}

fn extract_use(u: &syn::ItemUse) -> UseInfo {
    let mut bindings = Vec::new();
    collect_use_bindings(&u.tree, &mut Vec::new(), &mut bindings);
    UseInfo { path: use_tree_to_string(&u.tree), vis: visibility(&u.vis), bindings }
}

/// Flatten a `use` tree into the names it binds. `use a::{b, c as d}` binds `b`
/// to `a::b` and `d` to `a::c`; `use a::*` binds nothing under a name and is
/// recorded as a glob over `a`.
fn collect_use_bindings(tree: &syn::UseTree, prefix: &mut Vec<String>, out: &mut Vec<UseBindingInfo>) {
    match tree {
        syn::UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            collect_use_bindings(&p.tree, prefix, out);
            prefix.pop();
        }
        // `use a::b::{self}` binds `b` to `a::b`, not a name called "self".
        syn::UseTree::Name(n) if n.ident == "self" => {
            if let Some(parent) = prefix.last().cloned() {
                out.push(UseBindingInfo { local: Some(parent), path: prefix.clone() });
            }
        }
        syn::UseTree::Rename(r) if r.ident == "self" => {
            out.push(UseBindingInfo { local: Some(r.rename.to_string()), path: prefix.clone() });
        }
        syn::UseTree::Name(n) => {
            let mut path = prefix.clone();
            path.push(n.ident.to_string());
            out.push(UseBindingInfo { local: Some(n.ident.to_string()), path });
        }
        syn::UseTree::Rename(r) => {
            let mut path = prefix.clone();
            path.push(r.ident.to_string());
            out.push(UseBindingInfo { local: Some(r.rename.to_string()), path });
        }
        syn::UseTree::Glob(_) => {
            out.push(UseBindingInfo { local: None, path: prefix.clone() });
        }
        syn::UseTree::Group(g) => {
            for item in &g.items {
                collect_use_bindings(item, prefix, out);
            }
        }
    }
}

fn use_tree_to_string(tree: &syn::UseTree) -> String {
    match tree {
        syn::UseTree::Path(p) => format!("{}::{}", p.ident, use_tree_to_string(&p.tree)),
        syn::UseTree::Name(n) => n.ident.to_string(),
        syn::UseTree::Rename(r) => format!("{} as {}", r.ident, r.rename),
        syn::UseTree::Glob(_) => "*".to_string(),
        syn::UseTree::Group(g) => {
            let items: Vec<String> = g.items.iter().map(|t| use_tree_to_string(t)).collect();
            format!("{{{}}}", items.join(", "))
        }
    }
}

/// `#[from]` on a field of a `thiserror` enum. The attribute is thiserror's
/// instruction to write an `impl From` for the enum, and it implies `#[source]`
/// — which is why `#[source]` alone does not count here.
fn has_from_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("from"))
}

/// The module named by `#[serde(with = "..")]`, if the field carries one.
fn serde_with_attr(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let tokens = attr.meta.to_token_stream().to_string();
        if let Some(at) = tokens.find("with") {
            let rest = &tokens[at + 4..];
            let start = rest.find('"')? + 1;
            let end = rest[start..].find('"')? + start;
            return Some(rest[start..end].to_string());
        }
    }
    None
}

fn extract_fields(fields: &Fields) -> Vec<FieldInfo> {
    match fields {
        Fields::Named(named) => named.named.iter().map(|f| {
            FieldInfo {
                name: f.ident.as_ref().map(|i| name_map::to_camel_case(&i.to_string())),
                rust_name: f.ident.as_ref().map(|i| i.to_string()),
                rust_ty: f.ty.clone(),
                ty: None,
                is_pub: is_public(&f.vis),
                is_from: has_from_attr(&f.attrs),
                serde_with: serde_with_attr(&f.attrs),
            }
        }).collect(),
        Fields::Unnamed(unnamed) => unnamed.unnamed.iter().enumerate().map(|(i, f)| {
            FieldInfo {
                name: Some(format!("_{}", i)),
                rust_name: None,
                rust_ty: f.ty.clone(),
                ty: None,
                is_pub: is_public(&f.vis),
                is_from: has_from_attr(&f.attrs),
                serde_with: serde_with_attr(&f.attrs),
            }
        }).collect(),
        Fields::Unit => Vec::new(),
    }
}

/// The generic parameter names a declaration introduces, in order. The engine
/// needs these to tell a `T` in a written type from a type called `T`.
fn type_param_names(generics: &syn::Generics) -> Vec<String> {
    generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Type(t) => Some(t.ident.to_string()),
            _ => None,
        })
        .collect()
}

/// What each parameter falls back to when a use site leaves it unwritten.
/// `HashMap<K, V, S = RandomState>` is a three-parameter type that ankurah
/// always writes with two.
fn type_param_defaults(generics: &syn::Generics) -> Vec<Option<syn::Type>> {
    generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Type(t) => Some(t.default.clone()),
            _ => None,
        })
        .collect()
}

fn extract_generics(generics: &syn::Generics) -> String {
    if generics.params.is_empty() {
        return String::new();
    }

    let params: Vec<String> = generics.params.iter().filter_map(|p| {
        match p {
            syn::GenericParam::Type(t) => {
                let name = t.ident.to_string();
                let bounds: Vec<String> = t.bounds.iter().filter_map(|b| {
                    if let syn::TypeParamBound::Trait(trait_bound) = b {
                        let trait_name = trait_bound.path.segments.last()?.ident.to_string();
                        if matches!(trait_name.as_str(), "Send" | "Sync" | "Sized") {
                            return None;
                        }
                        Some(trait_name)
                    } else {
                        None
                    }
                }).collect();

                let default_part = t.default.as_ref().map(|d| {
                    format!(" = {}", name_map::map_type(d))
                }).unwrap_or_default();

                if bounds.is_empty() {
                    Some(format!("{}{}", name, default_part))
                } else {
                    Some(format!("{} extends {}{}", name, bounds.join(" & "), default_part))
                }
            }
            syn::GenericParam::Lifetime(_) => None,
            // A const generic is a value in Rust and a type in TypeScript:
            // `IVec<T, 3>` is written against a numeric literal type, so the
            // parameter is bounded by `number`. Writing `N: number` — Rust's
            // own spelling — is not a TypeScript parameter at all, and the
            // stripped use site then read `IVec<T, N:>`.
            syn::GenericParam::Const(c) => {
                Some(format!("{} extends number", c.ident))
            }
        }
    }).collect();

    if params.is_empty() {
        String::new()
    } else {
        format!("<{}>", params.join(", "))
    }
}
