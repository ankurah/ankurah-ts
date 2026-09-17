//! Enum variants written as paths, and which of them the emitter gives a class.
//!
//! For: `Type::Variant` and an associated function are one spelling in Rust,
//! and the answer decides whether the emitter writes a constructor call or a
//! variant tag. The enum is resolved through its own path so that a crate type
//! sharing a leaf name with another is still its own type.

use crate::ty::Ty;

use super::context::TypeContext;

impl TypeContext<'_> {

    /// Is `Type::Variant` an enum variant, as opposed to an associated
    /// function? The enum is resolved through its own path, never by the last
    /// segment of it.
    pub fn is_variant(&self, type_path: &str, variant: &str) -> bool {
        let mut segments: Vec<String> = type_path
            .split('.')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        if segments.is_empty() {
            return false;
        }
        segments.push(variant.to_string());
        if self.registry.lookup_variant(self.module, &segments).is_some() {
            return true;
        }
        // The emitted name is the LEAF, because the port flattens a crate's
        // module tree into a package's exports: `ast::Literal::I64` is written
        // `Literal.I64` and imported from `./ast`. A module that says only
        // `use crate::ast;` has no `Literal` in scope, so asking from there
        // answers no — and the call was then written as an associated function
        // of a class, not as the variant it is. The crate root is where the
        // flattened surface lives, so it is asked second.
        let root = self.registry.crate_root_of(self.module);
        self.registry
            .modules()
            .ids()
            .filter(|m| self.registry.modules().is_within(*m, root))
            .any(|m| m != self.module && self.registry.lookup_variant(m, &segments).is_some())
    }

    /// The enum and variant a path names, where it names a *unit* variant of an
    /// enum this crate emits a class for.
    ///
    /// A unit variant in expression position is a value that has to be built —
    /// `new ParseError('Empty', {})` — exactly as a payload-carrying one is.
    /// Writing it as a member of the class instead named a static nothing
    /// declares, which reads `undefined` and compares unequal to every variant
    /// the same file constructs properly.
    pub fn unit_variant_of_emitted_enum(&self, segments: &[String]) -> Option<(String, String)> {
        let (id, variant) = self.registry.lookup_variant(self.module, segments)?;
        let ty = Ty::Named {
            id,
            args: Vec::new(),
        };
        if !crate::emit_impls::has_emitted_class(self.registry, &ty) {
            return None;
        }
        let def = self.registry.def(id)?;
        let crate::registry::TypeKind::Enum { variants } = &def.kind else {
            return None;
        };
        let found = variants.iter().find(|v| v.name == variant)?;
        if !found.fields.is_empty() {
            return None;
        }
        Some((self.registry.name_of(id), variant))
    }

    /// The enum and variant a path names, where the path names a variant of an
    /// enum this crate emits a class for — whether or not it carries fields.
    ///
    /// `unit_variant_of_emitted_enum` answers only for a variant with no
    /// payload, because a path in expression position is a VALUE only then. A
    /// struct-variant LITERAL — `Predicate::Comparison { left, .. }` — names a
    /// variant that does carry fields and is built the same way.
    pub fn variant_of_emitted_enum(&self, segments: &[String]) -> Option<(String, String)> {
        // `Self::Add { .. }` inside `impl WatcherChange` names the same variant
        // `WatcherChange::Add` does. `Self` is not a name the registry holds,
        // so the path was looked up, found nothing, and fell through to the
        // struct-literal writing, which emitted `new WatcherChange.Add(..)` —
        // not a constructor.
        let (id, variant) = match (segments.first().map(String::as_str), self.self_ty.as_ref()) {
            (Some("Self"), Some(Ty::Named { id, .. })) if segments.len() == 2 => {
                let variant = segments[1].clone();
                if !self.registry.is_variant_of(*id, &variant) {
                    return None;
                }
                (*id, variant)
            }
            _ => self.registry.lookup_variant(self.module, segments)?,
        };
        let ty = Ty::Named { id, args: Vec::new() };
        if !crate::emit_impls::has_emitted_class(self.registry, &ty) {
            return None;
        }
        Some((self.registry.name_of(id), variant))
    }

}
