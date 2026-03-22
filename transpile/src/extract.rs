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

/// Extract all items from a Rust source file.
/// If features is provided, #[cfg(...)] expressions are evaluated against it.
/// Otherwise, only wasm/uniffi are skipped (legacy behavior).
pub fn extract(path: &Path) -> Result<RustFile> {
    extract_with_features(path, None)
}

pub fn extract_with_features(path: &Path, features: Option<&crate::cfg::CfgFeatures>) -> Result<RustFile> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let syntax = syn::parse_file(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;

    let mut file = RustFile {
        path: path.display().to_string(),
        structs: Vec::new(),
        enums: Vec::new(),
        traits: Vec::new(),
        functions: Vec::new(),
        impls: Vec::new(),
        uses: Vec::new(),
        type_aliases: Vec::new(),
        consts: Vec::new(),
        test_functions: Vec::new(),
    };

    for item in &syntax.items {
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
                file.traits.push(extract_trait(t));
            }
            syn::Item::Fn(f) => {
                if is_skipped_cfg_with(&f.attrs, features) { continue; }
                file.functions.push(extract_fn_with_body(&f.sig, is_public(&f.vis), &f.attrs, Some(&f.block)));
            }
            syn::Item::Impl(i) => {
                if is_skipped_cfg_with(&i.attrs, features) { continue; }
                file.impls.push(extract_impl(i));
            }
            syn::Item::Use(u) => {
                file.uses.push(extract_use(u));
            }
            syn::Item::Type(t) => {
                if is_skipped_cfg_with(&t.attrs, features) { continue; }
                file.type_aliases.push(TypeAliasInfo {
                    name: t.ident.to_string(),
                    ty: name_map::map_type(&t.ty),
                    is_pub: is_public(&t.vis),
                });
            }
            syn::Item::Const(c) => {
                if is_skipped_cfg_with(&c.attrs, features) { continue; }
                file.consts.push(ConstInfo {
                    name: c.ident.to_string(),
                    ty: name_map::map_type(&c.ty),
                    is_pub: is_public(&c.vis),
                });
            }
            syn::Item::Mod(m) => {
                // Check for #[cfg(test)] mod tests { ... }
                if is_test_module(&m.attrs) {
                    if let Some((_, items)) = &m.content {
                        for item in items {
                            if let syn::Item::Fn(f) = item {
                                if is_test_fn(&f.attrs) {
                                    file.test_functions.push(extract_fn_with_body(&f.sig, true, &f.attrs, Some(&f.block)));
                                }
                            }
                        }
                    }
                }
                // Extract items from non-test cfg-gated mod blocks
                // (e.g., mod stack { ... } with #[cfg(feature = "singlethread")])
                else if !is_skipped_cfg_with(&m.attrs, features) {
                    if let Some((_, items)) = &m.content {
                        for item in items {
                            match item {
                                syn::Item::Fn(f) => {
                                    if !is_skipped_cfg_with(&f.attrs, features) {
                                        file.functions.push(extract_fn_with_body(&f.sig, is_public(&f.vis), &f.attrs, Some(&f.block)));
                                    }
                                }
                                syn::Item::Struct(s) => {
                                    if !is_skipped_cfg_with(&s.attrs, features) {
                                        file.structs.push(extract_struct(s));
                                    }
                                }
                                syn::Item::Enum(e) => {
                                    if !is_skipped_cfg_with(&e.attrs, features) {
                                        file.enums.push(extract_enum(e));
                                    }
                                }
                                syn::Item::Use(u) => {
                                    file.uses.push(extract_use(u));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(file)
}

fn is_public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

fn is_skipped_cfg(attrs: &[syn::Attribute]) -> bool {
    is_skipped_cfg_with(attrs, None)
}

fn is_skipped_cfg_with(attrs: &[syn::Attribute], features: Option<&crate::cfg::CfgFeatures>) -> bool {
    if let Some(features) = features {
        return crate::cfg::should_skip(attrs, features);
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
        fields: extract_fields(&s.fields),
        generics: extract_generics(&s.generics),
        derives: extract_derives(&s.attrs),
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
        VariantInfo {
            name: v.ident.to_string(),
            fields: extract_fields(&v.fields),
            is_serde_other,
        }
    }).collect();

    EnumInfo {
        name: e.ident.to_string(),
        is_pub: is_public(&e.vis),
        variants,
        generics: extract_generics(&e.generics),
        derives: extract_derives(&e.attrs),
    }
}

fn extract_trait(t: &syn::ItemTrait) -> TraitInfo {
    let mut has_default_impls = false;
    let methods = t.items.iter().filter_map(|item| {
        if let syn::TraitItem::Fn(method) = item {
            if method.default.is_some() {
                has_default_impls = true;
            }
            Some(extract_fn(&method.sig, true, &method.attrs))
        } else {
            None
        }
    }).collect();

    TraitInfo {
        name: t.ident.to_string(),
        is_pub: is_public(&t.vis),
        methods,
        has_default_impls,
        generics: extract_generics(&t.generics),
    }
}

/// Extract function with body — stores the raw syn::Block for deferred translation.
/// Body translation happens in Phase 3 (with full type context), not during extraction.
fn extract_fn_with_body(sig: &syn::Signature, is_pub: bool, attrs: &[syn::Attribute], body: Option<&syn::Block>) -> FnInfo {
    let mut info = extract_fn(sig, is_pub, attrs);
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
    extract_fn_with_body(sig, is_pub, attrs, body)
}

fn extract_fn(sig: &syn::Signature, is_pub: bool, attrs: &[syn::Attribute]) -> FnInfo {
    let rust_name = sig.ident.to_string();
    let ts_name = name_map::map_fn_name(&rust_name);
    let is_async = sig.asyncness.is_some();

    let mut is_static = true;
    let params: Vec<ParamInfo> = sig.inputs.iter().filter_map(|arg| {
        match arg {
            FnArg::Receiver(_r) => {
                is_static = false;
                None
            }
            FnArg::Typed(pat) => {
                let name = if let syn::Pat::Ident(ident) = &*pat.pat {
                    name_map::to_camel_case(&ident.ident.to_string())
                } else {
                    "arg".to_string()
                };
                Some(ParamInfo {
                    name,
                    ty: name_map::map_type(&pat.ty),
                    is_self: false,
                    is_mut_self: false,
                })
            }
        }
    }).collect();

    let return_type = match &sig.output {
        ReturnType::Default => "void".to_string(),
        ReturnType::Type(_, ty) => name_map::map_type(ty),
    };

    FnInfo {
        name: rust_name,
        ts_name,
        is_pub,
        is_async,
        is_static,
        params,
        return_type,
        generics: extract_generics(&sig.generics),
        is_test: is_test_fn(attrs),
        body_ast: None,
        body_ts: None,
    }
}

fn extract_impl(i: &syn::ItemImpl) -> ImplInfo {
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

    let (trait_name, trait_type_args) = if let Some((_, path, _)) = &i.trait_ {
        let name = path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
        let args = path.segments.last().map(|s| {
            if let syn::PathArguments::AngleBracketed(args) = &s.arguments {
                args.args.iter().filter_map(|a| {
                    if let syn::GenericArgument::Type(ty) = a {
                        Some(name_map::map_type(ty))
                    } else {
                        None
                    }
                }).collect()
            } else {
                Vec::new()
            }
        }).unwrap_or_default();
        (Some(name), args)
    } else {
        (None, Vec::new())
    };

    let methods = i.items.iter().filter_map(|item| {
        if let syn::ImplItem::Fn(method) = item {
            if is_skipped_cfg(&method.attrs) { return None; }
            Some(extract_fn_with_body_and_self(
                &method.sig, is_public(&method.vis), &method.attrs,
                Some(&method.block), &target_type))
        } else {
            None
        }
    }).collect();

    ImplInfo {
        target_type,
        trait_name,
        trait_type_args,
        methods,
    }
}

fn extract_use(u: &syn::ItemUse) -> UseInfo {
    UseInfo {
        path: use_tree_to_string(&u.tree),
        is_pub: is_public(&u.vis),
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

fn extract_fields(fields: &Fields) -> Vec<FieldInfo> {
    match fields {
        Fields::Named(named) => named.named.iter().map(|f| {
            FieldInfo {
                name: f.ident.as_ref().map(|i| name_map::to_camel_case(&i.to_string())),
                ty: name_map::map_type(&f.ty),
                rust_ty: f.ty.to_token_stream().to_string(),
                is_pub: is_public(&f.vis),
            }
        }).collect(),
        Fields::Unnamed(unnamed) => unnamed.unnamed.iter().enumerate().map(|(i, f)| {
            FieldInfo {
                name: Some(format!("_{}", i)),
                ty: name_map::map_type(&f.ty),
                rust_ty: String::new(),
                is_pub: is_public(&f.vis),
            }
        }).collect(),
        Fields::Unit => Vec::new(),
    }
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

                if bounds.is_empty() {
                    Some(name)
                } else {
                    Some(format!("{} extends {}", name, bounds.join(" & ")))
                }
            }
            syn::GenericParam::Lifetime(_) => None,
            syn::GenericParam::Const(c) => {
                Some(format!("{}: number", c.ident))
            }
        }
    }).collect();

    if params.is_empty() {
        String::new()
    } else {
        format!("<{}>", params.join(", "))
    }
}
