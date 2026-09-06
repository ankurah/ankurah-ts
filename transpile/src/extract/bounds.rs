//! What a type parameter's BOUND is worth, as the TypeScript the port writes.
//!
//! For: a bound names a TypeScript interface, and a trait the declared surface
//! holds has none for it to name — so a `where` clause's bounds are dropped
//! unless the port has a spelling of its own for them. Two it has. A CALLABLE
//! bound is `Invocable`, which carries the shape, so `invoke(f, x)` on a value
//! of that type answers the right type instead of `unknown`. An
//! `IntoIterator`/`Iterator` bound is `Iterable`, because the port writes such
//! a value as an ARRAY and a spread over a parameter with no bound at all is
//! `TS2488: Type 'I' must have a '[Symbol.iterator]()' method` (N11).

use crate::name_map;

/// `FnOnce(A) -> R` as the port's `Invocable<[A], R>`, which is what a bound
/// closure parameter really accepts: a plain function, or the `OwnedClosure`
/// the emitter writes when the closure captured values with drop glue.
pub(super) fn invocable_bound(trait_name: &str, bound: &syn::TraitBound) -> Option<String> {
    if !matches!(trait_name, "Fn" | "FnMut" | "FnOnce") {
        return None;
    }
    let segment = bound.path.segments.last()?;
    let syn::PathArguments::Parenthesized(args) = &segment.arguments else {
        return None;
    };
    let inputs: Vec<String> = args.inputs.iter().map(name_map::map_type).collect();
    let output = match &args.output {
        syn::ReturnType::Type(_, ty) => name_map::map_type(ty),
        syn::ReturnType::Default => "void".to_string(),
    };
    Some(format!("Invocable<[{}], {}>", inputs.join(", "), output))
}

/// Is this bound one the port has a TypeScript spelling for, so that a `where`
/// clause carrying it says something the emitted signature can hold?
///
/// The callables are `Invocable`, and `IntoIterator`/`Iterator` are `Iterable`:
/// the port writes such a value as an ARRAY, and a spread over a parameter
/// with no bound at all is `TS2488: Type 'I' must have a '[Symbol.iterator]()'
/// method`. Live at `ankql/ast.ts`'s `populate<I, V, E>(values: I)`, whose
/// `let valuesIter = [...values];` had nothing saying `I` could be spread
/// (N11). Every other trait bound would be new surface: a bound names a
/// TypeScript interface, and a trait the declared surface holds has none.
pub(super) fn is_callable_bound(bound: &syn::TypeParamBound) -> bool {
    let syn::TypeParamBound::Trait(trait_bound) = bound else {
        return false;
    };
    trait_bound.path.segments.last().is_some_and(|s| {
        matches!(
            s.ident.to_string().as_str(),
            "Fn" | "FnMut" | "FnOnce" | "IntoIterator" | "Iterator"
        )
    })
}

/// `I: IntoIterator<Item = V>` as the `Iterable<V>` the port's spelling needs.
///
/// The port materialises an iterator as an array, so what a bounded parameter
/// has to promise is that it can be spread. `Item` is written as an associated
/// binding — `IntoIterator<Item = V>` — and where the source does not write one
/// the element is unknown, which `Iterable<unknown>` says exactly.
pub(super) fn iterable_bound(trait_name: &str, bound: &syn::TraitBound) -> Option<String> {
    if !matches!(trait_name, "IntoIterator" | "Iterator") {
        return None;
    }
    let item = bound
        .path
        .segments
        .last()
        .and_then(|segment| match &segment.arguments {
            syn::PathArguments::AngleBracketed(args) => Some(args),
            _ => None,
        })
        .and_then(|args| {
            args.args.iter().find_map(|arg| match arg {
                syn::GenericArgument::AssocType(assoc) if assoc.ident == "Item" => {
                    Some(name_map::map_type(&assoc.ty))
                }
                _ => None,
            })
        })
        .unwrap_or_else(|| "unknown".to_string());
    Some(format!("Iterable<{}>", item))
}
