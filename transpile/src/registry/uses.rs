//! What a `use` puts in scope.
//!
//! Two shapes need saying out loud. A `use` inside a function BODY is scoped by
//! Rust to the block it stands in, and the engine's binding table is per
//! module: the bindings are hoisted, but only where the module does not already
//! claim the name, because widening a name's scope must not change what another
//! body in the same module means by it. And `use path::Trait as _;` puts a
//! trait in scope for method resolution while binding no name at all — which is
//! why the scope test cannot be a lookup by name.

use super::module::{ModuleId, UseBinding};
use super::build::vis_of;
use super::{Callee, Ns, Probe, TypeRegistry};
use crate::diag::DiagSink;
use crate::types::RustFile;

/// The names a module's `use` items bind, hoisted body `use`s included.
pub(super) fn module_use_bindings(
    reg: &TypeRegistry,
    module: ModuleId,
    file: &RustFile,
    sink: &DiagSink,
) -> Vec<UseBinding> {
    // A module-level `use` binds its names outright. A `use` written inside a
    // body binds them too — Rust scopes it to the block, and the engine's table
    // is per module — but only where nothing at module level already claims the
    // name: widening a name's scope must not change what another body in the
    // same module means by it. A glob is never hoisted, because widening a
    // glob widens every name it could ever bring.
    let mut claimed: std::collections::HashSet<String> = std::collections::HashSet::new();
    for u in file.uses.iter().filter(|u| !u.from_body) {
        claimed.extend(u.bindings.iter().filter_map(|b| b.local.clone()));
    }
    let bindings: Vec<UseBinding> = file
        .uses
        .iter()
        .flat_map(|u| {
            let vis = vis_of(u.vis, module, reg, sink);
            let from_body = u.from_body;
            let claimed = &claimed;
            u.bindings.iter().filter_map(move |b| {
                if from_body {
                    let local = b.local.as_ref()?;
                    if claimed.contains(local) {
                        return None;
                    }
                }
                Some(UseBinding { local: b.local.clone(), path: b.path.clone(), vis })
            })
        })
        .collect();
    bindings
}

impl TypeRegistry {
    /// The leaf names this crate declares in MORE THAN ONE module.
    ///
    /// I11: the port flattens a crate's modules into one package surface, and a
    /// file's import list is keyed by the leaf — so a file that names
    /// `left::Wrap` and `right::Wrap` imports one of them and writes the bare
    /// name for both, and a signature against the other names the wrong class.
    /// The crate index already tells them apart (`left_Wrap`, `right_Wrap`);
    /// a file cannot yet, and the site says so rather than emitting a
    /// signature that is quietly about the other type.
    pub fn leaves_declared_twice(&self) -> std::collections::HashSet<String> {
        let mut by_leaf: std::collections::HashMap<&str, Vec<ModuleId>> =
            std::collections::HashMap::new();
        for def in &self.defs {
            // Only THIS crate's modules. An in-family crate is read for its
            // declarations under `crate::<name>::..`, so it is a CHILD of the
            // crate root — and a type it declares comes in through a package
            // import, which names the package and not a module of this one.
            // `ankurah_signals::ListenerGuard` in `core` is that shape, and
            // eighteen of the twenty first reports were it.
            if self.modules().get(def.module).is_system || self.is_in_a_sibling_crate(def.module) {
                continue;
            }
            by_leaf.entry(def.name.as_str()).or_default().push(def.module);
        }
        by_leaf
            .into_iter()
            .filter(|(_, modules)| {
                modules.len() > 1 && modules.iter().any(|m| *m != modules[0])
            })
            .map(|(name, _)| name.to_string())
            .collect()
    }

    /// Is this module inside an in-family crate read only for its declarations?
    fn is_in_a_sibling_crate(&self, module: ModuleId) -> bool {
        let roots = self.sibling_crate_roots();
        let mut at = module;
        for _ in 0..64 {
            if roots.contains(&at) {
                return true;
            }
            match self.modules().get(at).parent {
                Some(parent) => at = parent,
                None => return false,
            }
        }
        false
    }
}

impl Probe<'_> {
    /// Is the trait this callee came from nameable from the module that wrote
    /// the call?
    ///
    /// Rust needs the trait in scope for the method to exist at all. The engine
    /// only *reports* a sole candidate whose trait it cannot name, rather than
    /// deleting the method: the answer would then depend on the `use` map being
    /// complete, and a gap there would silently remove a method instead of
    /// showing up in the diagnostics. Where two candidates compete it does
    /// decide, because there the answer turns on it.
    pub(super) fn trait_in_scope(&self, callee: &Callee) -> bool {
        // A trait the RECEIVER'S OWN TYPE names is in scope for the methods it
        // declares, whatever the module imported: `where S: serde::Serializer`
        // is what makes `serializer.is_human_readable()` a call at all, and a
        // `dyn Trait` or an `impl Trait` says the same at its own type. Asking
        // the module's `use` list about one answers a different question — it
        // reported six calls in proto's four serde impls, and the import it
        // asked for is one Rust does not need. `Callee::TraitObject` is the
        // callee `bound_picks` writes and the only one those three produce.
        if matches!(callee, Callee::TraitObject(..)) {
            return true;
        }
        let Some(trait_id) = self.trait_of(callee) else {
            return true;
        };
        let name = self.reg.name_of(trait_id);
        if matches!(
            self.reg.lookup(self.module, Ns::Type, &[name.clone()]),
            Ok(Some(super::Def::Type(found))) if found == trait_id
        ) {
            return true;
        }
        // `use base64::Engine as _;` puts the trait in scope for method
        // resolution and binds NO name — which is the whole point of writing
        // it that way, and why a lookup by name cannot find it. `proto`'s
        // `data.rs` and `id.rs` write exactly that, and every `encode`/`decode`
        // call under it was reported as reaching a trait nobody had imported.
        self.reg
            .modules()
            .get(self.module)
            .uses
            .iter()
            .any(|u| u.local.as_deref() == Some("_") && u.path.last().map(String::as_str) == Some(name.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

    /// `core/selection/filter.rs`'s `compare_values_with_cast` writes
    /// `use crate::value::ValueType;` in its BODY. Rust scopes that to the
    /// block; the engine's binding table is per module, so the name bound
    /// nothing, `ValueType::of(l) == ValueType::of(r)` had no type on either
    /// side, and the equality was left as `===` between two freshly built
    /// objects — always false, with no diagnostic, because a gap is reported
    /// where a name is BOUND and this one was bound nowhere.
    #[test]
    fn a_type_named_by_a_body_use_is_resolved() {
        let mut c = Fixture::build(&[
            ("lib.rs", "pub mod kind;\npub mod ask;\n"),
            (
                "kind.rs",
                "#[derive(Clone, Copy, PartialEq, Eq)]\n\
                 pub enum Kind { Small, Large }\n\
                 pub struct N { pub n: u32 }\n\
                 impl Kind { pub fn of(v: &N) -> Kind { if v.n == 0 { Kind::Small } else { Kind::Large } } }",
            ),
            (
                "ask.rs",
                "use crate::kind::N;\n\
                 pub fn same(l: &N, r: &N) -> bool {\n\
                 use crate::kind::Kind;\n\
                 Kind::of(l) == Kind::of(r) }",
            ),
        ]);
        let ts = c.translated_method("ask.rs", "same");
        assert!(ts.contains("Kind.of(l).equals(Kind.of(r))"), "{}", ts);
    }

    /// Hoisting widens a name's scope from the block to the module, so it must
    /// not overrule a name the module already binds: `use crate::far::Kind;`
    /// at module level wins over a body `use crate::near::Kind;` for every
    /// other body in the file.
    #[test]
    fn a_body_use_never_overrules_the_modules_own_binding() {
        let mut c = Fixture::build(&[
            ("lib.rs", "pub mod far;\npub mod near;\npub mod ask;\n"),
            ("far.rs", "pub struct Kind { pub far: u32 }\n"),
            ("near.rs", "pub struct Kind { pub near: bool }\n"),
            (
                "ask.rs",
                "use crate::far::Kind;\n\
                 pub fn reach(k: &Kind) -> u32 { k.far }\n\
                 pub fn other() -> bool {\n\
                 use crate::near::Kind;\n\
                 let k = Kind { near: true };\n\
                 k.near }",
            ),
        ]);
        let ts = c.translated_method("ask.rs", "reach");
        assert!(ts.contains("k.far"), "the module's own binding still answers:\n{}", ts);
    }

    /// `use base64::Engine as _;` puts the trait in scope for method resolution
    /// and binds no name, which is the whole point of writing it that way.
    /// `proto`'s `data.rs` and `id.rs` write exactly that, and every
    /// `encode`/`decode` under it was reported as reaching a trait nobody had
    /// imported.
    #[test]
    fn an_anonymous_trait_import_puts_the_trait_in_scope() {
        let mut c = Fixture::build(&[
            ("lib.rs", "pub mod codec;\npub mod ask;\n"),
            (
                "codec.rs",
                "pub struct Engine;\n\
                 pub trait Encode { fn encode(&self) -> String; }\n\
                 impl Encode for Engine { fn encode(&self) -> String { String::new() } }",
            ),
            (
                "ask.rs",
                "use crate::codec::Engine;\n\
                 pub fn go(e: &Engine) -> String {\n\
                 use crate::codec::Encode as _;\n\
                 e.encode() }",
            ),
        ]);
        let _ = c.translated_method("ask.rs", "go");
        assert!(
            !c.messages().iter().any(|m| m.contains("which is not in scope here")),
            "the anonymous import IS the trait being in scope: {:?}",
            c.messages()
        );
    }
}

#[cfg(test)]
mod alias_tests {
    use crate::testing::Fixture;

    /// An ALIASED `use` binds a type under a name it is not declared with, and
    /// the port writes a type under the name it IS declared with — that is
    /// what its class is called and what the import list names.
    ///
    /// `use crate::value::VT as Outer;` emitted `Outer.of(n)`: a name nothing
    /// declares, nothing imports, and nothing reported.
    #[test]
    fn an_aliased_use_is_written_under_the_declared_name() {
        let mut c = Fixture::build(&[
            ("lib.rs", "pub mod value;\nuse crate::value::VT as Outer;\npub fn aliased(n: u32) -> u32 { Outer::of(n).n }"),
            ("value.rs", "pub struct VT { pub n: u32 }\nimpl VT { pub fn of(n: u32) -> VT { VT { n } } }"),
        ]);
        let ts = c.emitted("lib.rs");
        assert!(ts.contains("VT.of(n)"), "{ts}");
        assert!(!ts.contains("Outer"), "the alias reached the emitted text:\n{ts}");
        // The import list follows on its own, because it reads what the
        // emission writes: a batch run over the same two files opens
        // `import { VT } from './value';`. (This fixture does not resolve
        // imports, so it says `// TODO imports: VT` instead.)
    }
}

#[cfg(test)]
mod bound_tests {
    use crate::testing::Fixture;

    /// A trait the RECEIVER'S OWN TYPE names is in scope for the methods it
    /// declares, whatever the module imported.
    ///
    /// `where S: serde::Serializer` is what makes `serializer.is_human_readable()`
    /// a call at all. Asking the module's `use` list about it answers a
    /// different question, and reported six calls in proto's four serde impls,
    /// one in signals and thirty-one in core — each asking for an import Rust
    /// does not need.
    #[test]
    fn a_trait_named_by_a_bound_is_in_scope_for_its_own_methods() {
        let mut c = Fixture::build(&[
            ("lib.rs", "pub mod tell;\npub mod ask;\n"),
            ("tell.rs", "pub trait Tell { fn tell(&self) -> u32; }"),
            (
                "ask.rs",
                "pub fn ask<T>(t: &T) -> u32 where T: crate::tell::Tell { t.tell() }",
            ),
        ]);
        let ts = c.emitted("ask.rs");
        assert!(ts.contains("t.tell()"), "{ts}");
        assert!(
            c.messages().iter().all(|m| !m.contains("not in scope here")),
            "the bound IS the declaration: {:?}",
            c.messages()
        );
    }
}
