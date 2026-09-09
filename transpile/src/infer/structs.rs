//! What a struct literal is, and what its fields say about it.
//!
//! For: `Chunk { items }` written for a `Chunk<T>` takes its argument from the
//! field it was handed, and that field's own type may be an unknown the
//! declaration settles. One constraint per field carries both directions.

use syn::spanned::Spanned;

use super::context::TypeContext;
use super::shapes::member_name;
use crate::diag::Diag;
use crate::ty::subst::Subst;
use crate::ty::Ty;

impl TypeContext<'_> {
    /// The type a struct literal builds.
    ///
    /// A literal that leaves the declaration's arguments off stands for one
    /// unknown each, and each field constrains them.
    pub(crate) fn resolve_struct_literal(&self, lit: &syn::ExprStruct) -> Result<Ty, Diag> {
        let ty = syn::Type::Path(syn::TypePath {
            qself: lit.qself.clone(),
            path: lit.path.clone(),
        });
        // A struct literal may also name an enum variant: `Signal::Memo { .. }`.
        let segments: Vec<String> = lit
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        if let Some((id, _)) = self.registry.lookup_variant(self.module, &segments) {
            let params = self
                .registry
                .def(id)
                .map(|d| d.type_params.clone())
                .unwrap_or_default();
            return Ok(Ty::Named {
                id,
                args: params.into_iter().map(Ty::Param).collect(),
            });
        }

        let resolved = self.resolve_written_type_open(&ty)?;
        let Ty::Named { id, args } = &resolved else {
            return Ok(resolved);
        };
        let Some(def) = self.registry.def(*id) else {
            return Ok(resolved);
        };
        let subst: Subst = def
            .type_params
            .iter()
            .cloned()
            .zip(args.iter().cloned())
            .collect();
        let fields = def.fields.clone();
        for field in &lit.fields {
            let name = member_name(&field.member);
            let Some((_, declared)) = fields.iter().find(|(n, _)| *n == name) else {
                continue;
            };
            let declared = declared.substitute(&subst);
            let Some(actual) = self.actual_of(&field.expr) else { continue };
            // Emission erases `&`, so a field stands for what it refers to.
            self.constrain_here(Spanned::span(field), &declared, &actual);
        }
        Ok(resolved)
    }
}
