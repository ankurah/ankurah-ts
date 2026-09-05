//! A written type as RUST spells it, beside the TypeScript spelling the rest of
//! this module produces.
//!
//! For: R8 names a contested conversion by the source type Rust wrote, and the
//! TypeScript spelling erases the difference the impls are written for —
//! `i64`, `i32` and `f64` are all `number`, and `Vec<u32>` and `Vec<i32>` are
//! both `number[]`. Read through TypeScript, several impls with several bodies
//! looked like one and all but the last were dropped.

/// A written type as RUST spells it, leaf by leaf.
///
/// Not `map_type`: this is what tells two conversion sources apart, and the
/// TypeScript spelling erases the difference the impls are written for. Only
/// the parts a source type can be made of are rendered — a path, a reference, a
/// slice, an array, a tuple — and anything else falls back to `map_type`, which
/// is what it had before.
pub fn rust_spelling(ty: &syn::Type) -> String { spelling(ty, false) }

/// `qualify` says whether a path keeps its module segments: not at the TOP
/// level, where the caller writes them itself, and yes INSIDE a generic
/// argument, where they are the only thing telling two source types apart —
/// `From<Vec<a::Item>>` and `From<Vec<b::Item>>` were one spelling and the
/// second impl was dropped.
fn spelling(ty: &syn::Type, qualify: bool) -> String {
    match ty {
        syn::Type::Reference(r) => format!("&{}", spelling(&r.elem, qualify)),
        syn::Type::Slice(s) => format!("[{}]", spelling(&s.elem, qualify)),
        syn::Type::Array(a) => {
            let len = quote::ToTokens::to_token_stream(&a.len).to_string().replace(' ', "");
            format!("[{}; {}]", spelling(&a.elem, qualify), len)
        }
        syn::Type::Paren(p) => spelling(&p.elem, qualify),
        syn::Type::Group(g) => spelling(&g.elem, qualify),
        syn::Type::Tuple(t) => {
            format!("({})", t.elems.iter().map(|e| spelling(e, qualify)).collect::<Vec<_>>().join(", "))
        }
        syn::Type::Path(p) => {
            let Some(segment) = p.path.segments.last() else {
                return crate::name_map::map_type(ty);
            };
            let all: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
            let leaf = if qualify { all.join("::") } else { segment.ident.to_string() };
            match &segment.arguments {
                syn::PathArguments::None => leaf,
                syn::PathArguments::AngleBracketed(args) => {
                    let inner: Vec<String> = args
                        .args
                        .iter()
                        .filter_map(|a| match a {
                            syn::GenericArgument::Type(t) => Some(spelling(t, true)),
                            _ => None,
                        })
                        .collect();
                    if inner.is_empty() {
                        leaf
                    } else {
                        format!("{}<{}>", leaf, inner.join(", "))
                    }
                }
                syn::PathArguments::Parenthesized(_) => crate::name_map::map_type(ty),
            }
        }
        other => crate::name_map::map_type(other),
    }
}

/// A conversion source as RUST wrote it: the leaf's own identifier, with the
/// module segments in front of it and a `&` kept.
///
/// R8 names a contested conversion by the source type Rust wrote, so this is
/// the spelling both halves of the naming agree on. A leaf that carries type
/// arguments keeps the spelling that shows what they are, because there is no
/// method name to build out of one either way.
pub fn rust_source_path(ty: &syn::Type) -> String {
    let (inner, borrowed) = match ty {
        syn::Type::Reference(r) => (&*r.elem, "&"),
        other => (other, ""),
    };
    let syn::Type::Path(path) = inner else {
        return format!("{}{}", borrowed, crate::name_map::map_type(inner));
    };
    let Some(last) = path.path.segments.last() else {
        return format!("{}{}", borrowed, crate::name_map::map_type(inner));
    };
    let leaf = match last.arguments {
        syn::PathArguments::None => last.ident.to_string(),
        _ => crate::name_map::map_type(inner),
    };
    if path.path.segments.len() < 2 {
        return format!("{}{}", borrowed, leaf);
    }
    let qualifier: Vec<String> = path
        .path
        .segments
        .iter()
        .take(path.path.segments.len() - 1)
        .map(|s| s.ident.to_string())
        .collect();
    format!("{}{}::{}", borrowed, qualifier.join("::"), leaf)
}
