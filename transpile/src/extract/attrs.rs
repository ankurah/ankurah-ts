//! The attributes extraction reads off a field.
//!
//! `#[from]` and `#[source]` are `thiserror`'s, and say which field a generated
//! `From` takes and which one `source()` answers. `#[serde(with = "..")]`
//! names the module a field is read and written through, which the codec has to
//! honour or the wire bytes differ.

use quote::ToTokens;

/// `#[from]` on a field of a `thiserror` enum. The attribute is thiserror's
/// instruction to write an `impl From` for the enum, and it implies `#[source]`
/// — which is why `#[source]` alone does not count here.
pub(super) fn has_from_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("from"))
}


/// Is this the error the variant wraps? `#[from]` implies `#[source]`, which is
/// how thiserror reads it.
pub(super) fn has_source_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("source") || a.path().is_ident("from"))
}


/// The module named by `#[serde(with = "..")]`, if the field carries one.
pub(super) fn serde_with_attr(attrs: &[syn::Attribute]) -> Option<String> {
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
