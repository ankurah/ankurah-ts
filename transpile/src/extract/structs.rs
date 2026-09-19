//! A `struct` item, read for everything the emitters need from it.

use super::{extract_derives, extract_fields, extract_generics, expanded_attrs, has_serde_flag,
    is_public, type_param_defaults, type_param_names, visibility};
use crate::types::{Constructor, StructInfo};

/// How a declaration lets its name be WRITTEN, which decides whether that name
/// alone is a value. A struct and an enum variant take the same three forms.
pub(crate) fn constructor_of(fields: &syn::Fields) -> Constructor {
    match fields {
        syn::Fields::Named(_) => Constructor::Braced,
        syn::Fields::Unnamed(_) => Constructor::Tuple,
        syn::Fields::Unit => Constructor::Unit,
    }
}

pub(super) fn extract_struct(s: &syn::ItemStruct, features: Option<&crate::cfg::CfgFeatures>) -> StructInfo {
    let attrs = expanded_attrs(&s.attrs, features, s.ident.span());
    StructInfo {
        name: s.ident.to_string(),
        is_pub: is_public(&s.vis),
        vis: visibility(&s.vis),
        fields: extract_fields(&s.fields, features),
        generics: extract_generics(&s.generics),
        type_params: type_param_names(&s.generics),
        syn_generics: s.generics.clone(),
        param_defaults: type_param_defaults(&s.generics),
        derives: extract_derives(&attrs),
        serde_transparent: has_serde_flag(&attrs, "transparent"),
        constructor: constructor_of(&s.fields),
        span: s.ident.span(),
    }
}
