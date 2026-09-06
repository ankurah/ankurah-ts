//! What a `use` puts in scope.
//!
//! Two shapes need saying out loud. A `use` inside a function BODY is scoped by
//! Rust to the block it stands in, and the engine's binding table is per
//! module, so a body `use` bound nothing at all and every name it introduced
//! resolved to whatever the module already had, or to nothing.
//! `core/selection/filter.rs`'s `compare_values_with_cast` writes
//! `use crate::value::ValueType;` in its body: `ValueType::of(l) ==
//! ValueType::of(r)` therefore had no type on either side, the equality was
//! left as `===` between two fresh objects — always false — and no diagnostic
//! was filed, because an operand the engine could not name reports where the
//! name was BOUND and nothing bound this one. A body `use` under an alias was
//! worse: `use crate::value::VT as Inner;` emitted `Inner.of(l)`, a name
//! nothing declares.
//!
//! So the bindings are hoisted — but only where the module does not already
//! claim the name, by BINDING it or by DECLARING it, because widening a name's
//! scope must not change what another body in the same module means by it, nor
//! what the module's own declaration means. A name two different bodies bring
//! in from two different places is claimed by neither, and both are reported.
//!
//! And `use path::Trait as _;` puts a trait in scope for method resolution
//! while binding no name at all — which is why the scope test cannot be a
//! lookup by name.

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
    // is per module — but only where the module does not already claim the
    // name, by BINDING it or by DECLARING it: widening a name's scope must not
    // change what another body in the same module means by it, and it must not
    // change what the module's own `Kind` means either. A glob is never
    // hoisted, because widening a glob widens every name it could ever bring.
    //
    // E8: `claimed` was built from the module's other `use` items alone, so a
    // module declaring `pub struct Kind` whose body wrote
    // `use crate::far::Kind;` had the far one hoisted over its own — and every
    // `Kind` in the file then resolved to a type with different fields.
    let mut claimed = claimed_names(file);
    // And a name TWO bodies bring in from different places is claimed by
    // neither: the module's table has one entry per name, so hoisting both
    // would leave the second body meaning the first body's type. Rust scopes
    // each to its own block and the port has no scope to put them in, so both
    // are reported and neither is hoisted (§3.6).
    for local in contested_body_names(file) {
        sink.report(
            proc_macro2::Span::call_site(),
            format!(
                "two function bodies in this module write `use` for different types both \
                 named `{}`, and the port has one binding table per module: neither is \
                 hoisted, and each body's `{}` is written from whatever else the module \
                 says it is",
                local, local
            ),
        );
        claimed.insert(local);
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

/// The names the module claims without any body's help: what its module-level
/// `use` items bind, and what it DECLARES.
fn claimed_names(file: &RustFile) -> std::collections::HashSet<String> {
    let mut claimed: std::collections::HashSet<String> = std::collections::HashSet::new();
    for u in file.uses.iter().filter(|u| !u.from_body) {
        claimed.extend(u.bindings.iter().filter_map(|b| b.local.clone()));
    }
    claimed.extend(file.structs.iter().map(|s| s.name.clone()));
    claimed.extend(file.enums.iter().map(|e| e.name.clone()));
    claimed.extend(file.traits.iter().map(|t| t.name.clone()));
    claimed.extend(file.type_aliases.iter().map(|a| a.name.clone()));
    claimed.extend(file.consts.iter().map(|c| c.name.clone()));
    claimed.extend(file.functions.iter().map(|f| f.name.clone()));
    claimed.extend(file.mod_decls.iter().map(|(name, _)| name.clone()));
    claimed.extend(file.inline_modules.iter().map(|(name, _)| name.clone()));
    claimed
}

/// The names two different function BODIES bring in from two different paths.
fn contested_body_names(file: &RustFile) -> Vec<String> {
    let mut seen: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for u in file.uses.iter().filter(|u| u.from_body) {
        for b in &u.bindings {
            let Some(local) = &b.local else { continue };
            let path = b.path.join("::");
            let paths = seen.entry(local.clone()).or_default();
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
    }
    let mut contested: Vec<String> =
        seen.into_iter().filter(|(_, paths)| paths.len() > 1).map(|(name, _)| name).collect();
    contested.sort();
    contested
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

    /// E8: a `use` inside a body is hoisted only where the module does not
    /// already claim the name — by BINDING it or by DECLARING it. `claimed`
    /// was built from the module's other `use` items alone, which is not what
    /// either doc comment said.
    #[test]
    fn a_body_use_does_not_hoist_over_a_name_the_module_declares() {
        let f = Fixture::build(&[
            (
                "lib.rs",
                "pub mod far;\n\
                 pub struct Kind { pub here: u32 }\n\
                 pub fn f() -> u32 { use crate::far::Kind; let k = Kind { there: 1 }; k.there }\n",
            ),
            ("far.rs", "pub struct Kind { pub there: u32 }\n"),
        ]);
        let module = f.module("lib.rs");
        let here = f.reg.module_type(module, "Kind").expect("the module declares Kind");
        // The module's own `Kind` is what the name means, and the body's `use`
        // did not widen the far one over it.
        assert_eq!(f.reg.name_of(here), "Kind");
        let fields = f.reg.def(here).expect("declared").fields.clone();
        assert!(
            fields.iter().any(|(name, _)| name == "here"),
            "the module's own Kind is what the table holds: {:?}",
            fields
        );
    }

    /// §3.6: a name TWO bodies bring in from different places is claimed by
    /// neither, and BOTH are reported. Hoisting both left the module's one
    /// binding table holding the first, so the second body silently meant the
    /// first body's type — `new Wrap(undefined)` with only one site reported.
    #[test]
    fn two_bodies_importing_the_same_leaf_are_both_reported() {
        let f = Fixture::build(&[
            (
                "lib.rs",
                "pub mod left;\n\
                 pub mod right;\n\
                 pub fn one() -> u32 { use crate::left::Wrap; let w = Wrap { a: 1 }; w.a }\n\
                 pub fn two() -> u32 { use crate::right::Wrap; let w = Wrap { b: 2 }; w.b }\n",
            ),
            ("left.rs", "pub struct Wrap { pub a: u32 }\n"),
            ("right.rs", "pub struct Wrap { pub b: u32 }\n"),
        ]);
        let reported = f.messages();
        assert!(
            reported.iter().any(|d| d.contains("two function bodies in this module write `use`")),
            "the clash is reported: {:?}",
            reported
        );
        // And neither is hoisted, so the module's table binds no `Wrap` at all.
        assert!(
            f.reg.module_type(f.module("lib.rs"), "Wrap").is_none(),
            "a body use was hoisted anyway"
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
