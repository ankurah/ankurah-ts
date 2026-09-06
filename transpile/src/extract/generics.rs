//! The generic parameters a signature or a declaration introduces, and which
//! of them the port writes.
//!
//! For: a `T` in a written type is not a type called `T`, and only the
//! declaration that introduced it says which it is. These readers answer that
//! for a signature, a struct, an enum and a trait alike — the names in order,
//! what each falls back to when a use site leaves it unwritten, and the
//! TypeScript parameter list the emitter writes. What the port carries of a
//! bound is only what it has a spelling for, so the callable bounds and the
//! iterable bound come through `super::bounds` and everything else is dropped.

use super::bounds::{invocable_bound, is_callable_bound, iterable_bound};
use crate::name_map;

/// The generic parameter names a declaration introduces, in order. The engine
/// needs these to tell a `T` in a written type from a type called `T`.
pub(super) fn type_param_names(generics: &syn::Generics) -> Vec<String> {
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
pub(super) fn type_param_defaults(generics: &syn::Generics) -> Vec<Option<syn::Type>> {
    generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Type(t) => Some(t.default.clone()),
            _ => None,
        })
        .collect()
}

/// Every type parameter of this signature whose bounds are ONE callable bound,
/// by name, with the `Invocable<..>` it is written as.
pub(crate) fn callable_only_params(
    sig: &syn::Signature,
) -> std::collections::HashMap<String, String> {
    let arguments: Vec<syn::Type> = sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Typed(t) => Some((*t.ty).clone()),
            _ => None,
        })
        .collect();
    let returned = match &sig.output {
        syn::ReturnType::Type(_, ty) => Some((**ty).clone()),
        syn::ReturnType::Default => None,
    };
    callable_only_params_of(&sig.generics, &arguments, returned.as_ref())
}

/// The same, for a caller that kept the pieces rather than the signature.
pub(crate) fn callable_only_params_of(
    generics: &syn::Generics,
    arguments: &[syn::Type],
    returned: Option<&syn::Type>,
) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for param in &generics.params {
        let syn::GenericParam::Type(t) = param else { continue };
        let name = t.ident.to_string();
        let carried: Vec<&syn::TypeParamBound> = t
            .bounds
            .iter()
            .chain(where_bounds(generics, &name))
            .filter(|b| !is_marker_bound(b))
            .collect();
        if carried.len() != 1 {
            continue;
        }
        let syn::TypeParamBound::Trait(bound) = carried[0] else { continue };
        let Some(leaf) = bound.path.segments.last().map(|s| s.ident.to_string()) else {
            continue;
        };
        // Only where the parameter's NAME is used nowhere else: a signature
        // that mentions it in its return type — `map<Transform>(t: Transform)
        // -> Map<.., Transform>` — still needs the parameter declared.
        if mentions_beyond_one_parameter(arguments, returned, &name) {
            continue;
        }
        if let Some(spelling) = invocable_bound(&leaf, bound) {
            out.insert(name, spelling);
        }
    }
    out
}

/// The written type with its references peeled, for the questions emission has
/// already erased the reference from.
pub(crate) fn peel_written_refs(ty: &syn::Type) -> &syn::Type {
    match ty {
        syn::Type::Reference(r) => peel_written_refs(&r.elem),
        syn::Type::Paren(p) => peel_written_refs(&p.elem),
        syn::Type::Group(g) => peel_written_refs(&g.elem),
        other => other,
    }
}

/// Does this signature name the type parameter anywhere but as the whole type
/// of exactly one argument?
fn mentions_beyond_one_parameter(
    arguments: &[syn::Type],
    returned: Option<&syn::Type>,
    name: &str,
) -> bool {
    // A reference is peeled first: emission erases it, so `f: &mut F` and
    // `f: F` are the same TypeScript parameter and the same question is being
    // asked of both. Testing the WRITTEN type missed every `&F` and `&mut F` —
    // ankql's `Predicate::walk` kept `<F extends Invocable<..>>` and answered
    // `unknown` at six sites, which is precisely what this rule exists to stop.
    let is_the_parameter = |ty: &syn::Type| match peel_written_refs(ty) {
        syn::Type::Path(path) => path.path.get_ident().is_some_and(|i| i == name),
        _ => false,
    };
    let mut as_a_whole_argument = 0usize;
    for ty in arguments {
        if is_the_parameter(ty) {
            as_a_whole_argument += 1;
            continue;
        }
        if mentions(ty, name) {
            return true;
        }
    }
    if returned.is_some_and(|ty| mentions(ty, name)) {
        return true;
    }
    as_a_whole_argument != 1
}

/// Does this type mention the named type parameter anywhere inside it?
fn mentions(ty: &syn::Type, name: &str) -> bool {
    struct Named<'n> {
        name: &'n str,
        found: bool,
    }
    impl syn::visit::Visit<'_> for Named<'_> {
        fn visit_ident(&mut self, ident: &syn::Ident) {
            if ident == self.name {
                self.found = true;
            }
        }
    }
    let mut named = Named { name, found: false };
    syn::visit::Visit::visit_type(&mut named, ty);
    named.found
}

/// `Send`, `Sync`, `Sized` and a lifetime say nothing about the shape.
fn is_marker_bound(bound: &syn::TypeParamBound) -> bool {
    match bound {
        syn::TypeParamBound::Trait(t) => t
            .path
            .segments
            .last()
            .is_some_and(|s| matches!(s.ident.to_string().as_str(), "Send" | "Sync" | "Sized")),
        _ => true,
    }
}

/// The generics list with the callable-only parameters left out: they are
/// written as the parameter's type instead.
pub(super) fn extract_generics_without(
    generics: &syn::Generics,
    without: &std::collections::HashMap<String, String>,
) -> String {
    if without.is_empty() {
        return extract_generics(generics);
    }
    let mut kept = generics.clone();
    kept.params = generics
        .params
        .iter()
        .filter(|p| match p {
            syn::GenericParam::Type(t) => !without.contains_key(&t.ident.to_string()),
            _ => true,
        })
        .cloned()
        .collect();
    extract_generics(&kept)
}

pub(super) fn extract_generics(generics: &syn::Generics) -> String {
    if generics.params.is_empty() {
        return String::new();
    }

    let params: Vec<String> = generics.params.iter().filter_map(|p| {
        match p {
            syn::GenericParam::Type(t) => {
                let name = t.ident.to_string();
                // A `where` clause says the same thing as an inline bound, and
                // reading only the inline ones left `F: FnOnce(..)` written
                // there with no constraint at all — so `invoke(f, x)` in the
                // emitted body answered `unknown` and every use of the answer
                // was a type error.
                //
                // Only the CALLABLE bounds are read from the `where` clause.
                // Every other trait bound there would be new surface: a bound
                // names a TypeScript interface, and a trait the declared surface
                // holds has none for it to name.
                let carried: Vec<&syn::TypeParamBound> = t
                    .bounds
                    .iter()
                    .chain(where_bounds(generics, &name).filter(|b| is_callable_bound(b)))
                    .collect();
                let bounds: Vec<String> = carried.iter().filter_map(|b| {
                    if let syn::TypeParamBound::Trait(trait_bound) = b {
                        let trait_name = trait_bound.path.segments.last()?.ident.to_string();
                        if matches!(trait_name.as_str(), "Send" | "Sync" | "Sized") {
                            return None;
                        }
                        // R10: a callable bound is what the port's `Invocable`
                        // says — either a plain function or the `OwnedClosure`
                        // the emitter writes — and it carries the shape, so
                        // `invoke` on a value of this type answers the right
                        // type instead of `unknown`.
                        if let Some(invocable) = invocable_bound(&trait_name, trait_bound) {
                            return Some(invocable);
                        }
                        if let Some(iterable) = iterable_bound(&trait_name, trait_bound) {
                            return Some(iterable);
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

/// The bounds a `where` clause puts on one type parameter.
fn where_bounds<'g>(
    generics: &'g syn::Generics,
    name: &str,
) -> impl Iterator<Item = &'g syn::TypeParamBound> {
    let owned = name.to_string();
    generics
        .where_clause
        .iter()
        .flat_map(|clause| clause.predicates.iter())
        .filter_map(move |predicate| match predicate {
            syn::WherePredicate::Type(pt) => match &pt.bounded_ty {
                syn::Type::Path(path)
                    if path.qself.is_none()
                        && path.path.is_ident(&syn::Ident::new(
                            &owned,
                            proc_macro2::Span::call_site(),
                        )) =>
                {
                    Some(pt.bounds.iter())
                }
                _ => None,
            },
            _ => None,
        })
        .flatten()
}

