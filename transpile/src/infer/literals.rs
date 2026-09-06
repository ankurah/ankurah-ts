//! What a struct or variant LITERAL says about the values written in it.
//!
//! Rust checks each value against the field it stands beside, and the port
//! needs the same two answers: what each field's type is, so the value is
//! translated in a typed position, and what order the declaration puts its
//! fields in, because a ported struct is built through a constructor that takes
//! them in declaration order.

use super::context::TypeContext;
use crate::ty::Ty;

impl TypeContext<'_> {
    /// What each field of the struct a literal builds is declared to hold.
    ///
    /// The declaration is what the field's initialiser has to produce, so
    /// `Header { len: 1 }` writes the width `len` declares rather than the
    /// `i32` a bare literal defaults to (spec 4.6).
    pub fn struct_literal_field_types(&self, lit: &syn::ExprStruct) -> Vec<(String, Ty)> {
        let Ok(Ty::Named { id, args }) = self.resolve_struct_literal(lit) else {
            return Vec::new();
        };
        let Some(def) = self.registry.def(id) else {
            return Vec::new();
        };
        let subst = crate::ty::bind_params(&def.type_params, &args);
        // `Enum::Variant { field: .. }` resolves to the ENUM, whose own `fields`
        // are empty — a variant's fields live on the variant. Without this the
        // values in a variant literal stood in no typed position at all, and a
        // `collect()` among them had no target: live at `core/retrieval.rs`'s
        // `NodeRequestBody::GetEvents { event_ids: ..collect() }` and
        // `core/peer_subscription/server.rs`'s `SubscriptionUpdate { items: .. }`.
        let fields = match &def.kind {
            crate::registry::TypeKind::Enum { variants } => {
                let named = lit.path.segments.last().map(|s| s.ident.to_string());
                match named.and_then(|v| variants.iter().find(|d| d.name == v)) {
                    Some(variant) => &variant.fields,
                    None => return Vec::new(),
                }
            }
            _ => &def.fields,
        };
        fields
            .iter()
            .map(|(name, ty)| (name.clone(), ty.substitute(&subst)))
            .collect()
    }

    /// The fields a struct literal's DECLARATION has, in declaration order,
    /// without resolving the literal's type arguments.
    ///
    /// `Attested { payload, attestations }` writes no `T`, so resolving the
    /// whole type fails and `struct_literal_field_types` answers nothing — but
    /// the ORDER of the fields does not depend on the arguments, and the order
    /// is what the constructor call needs.
    pub fn struct_literal_field_order(&self, lit: &syn::ExprStruct) -> Vec<String> {
        let segments: Vec<String> = lit
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        let mut id = match self.registry.lookup_type(self.module, &segments) {
            Ok(Some(crate::registry::Def::Type(id))) => Some(id),
            _ => None,
        };
        if id.is_none() {
            // `Self { .. }`, and a path naming a variant of an enum.
            if segments.len() == 1 && segments[0] == "Self" {
                if let Some(Ty::Named { id: self_id, .. }) = self.self_ty.as_ref() {
                    id = Some(*self_id);
                }
            }
        }
        let Some(id) = id else { return Vec::new() };
        let Some(def) = self.registry.def(id) else {
            return Vec::new();
        };
        def.field_order.clone()
    }
}
