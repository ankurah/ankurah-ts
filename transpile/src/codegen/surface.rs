//! Which names a crate's package surface offers, and what to do when two
//! modules offer the same one.
//!
//! For: Rust keeps `signals::broadcast::ListenerGuard` and
//! `signals::signal::ListenerGuard` apart because a module is a namespace. The
//! port has no nested namespace to mirror, so `index.ts` flattens every public
//! child module into the package with `export * from './broadcast'`. Two star
//! exports offering one name is an AMBIGUOUS export in JavaScript: the name is
//! not exported at all, silently — no error from tsc, no error from a bundler,
//! just a missing import at the far end. `ListenerGuard` was reachable from
//! `@ankurah/signals` in neither spelling.
//!
//! An explicit `export { X } from './m'` shadows every star export of `X`, so
//! the ambiguity is resolvable without dropping anything: the module Rust
//! itself reaches unqualified keeps the bare name, and every other module that
//! offers it keeps its own name under a module qualifier. What cannot be
//! resolved is reported rather than left to vanish.

use crate::registry::{Def, ModuleId, Ns, TypeRegistry, Vis};
use std::collections::{BTreeMap, BTreeSet};

/// Every name `export * from '<module>'` contributes to the file above it, and
/// the declaration each one resolves to.
///
/// That is the module's own public items plus whatever its own index re-exports
/// — its public children (flattened the same way) and the names its `pub use`
/// lines bring in. The DECLARATION matters, not only the name: `lineage.rs`
/// writes `pub use crate::retrieval::GetEvents;`, so two of core's modules offer
/// `GetEvents` and both mean the one trait, which JavaScript resolves without
/// complaint. A module is visited once, so a cycle of re-exports ends.
pub fn star_surface(reg: &TypeRegistry, module: ModuleId) -> BTreeMap<String, Def> {
    let mut out = BTreeMap::new();
    let mut seen = BTreeSet::new();
    collect(reg, module, &mut out, &mut seen);
    out
}

fn collect(
    reg: &TypeRegistry,
    module: ModuleId,
    out: &mut BTreeMap<String, Def>,
    seen: &mut BTreeSet<u32>,
) {
    if !seen.insert(module.0) {
        return;
    }
    let def = reg.modules().get(module);
    for ((_, name), item) in &def.items {
        if item.vis == Vis::Public {
            out.entry(name.clone()).or_insert(item.def);
        }
    }
    // A public child module is star-exported by this module's own index, so its
    // surface is part of what the module above sees.
    for child in def.children.values() {
        if reg.modules().get(*child).vis == Vis::Public {
            collect(reg, *child, out, seen);
        }
    }
    for u in &def.uses {
        if u.vis != Vis::Public {
            continue;
        }
        match (&u.local, &u.path[..]) {
            // `pub use signal::ListenerGuard;` — resolved, so that the same
            // declaration reached through two modules is one declaration.
            (Some(local), path) => {
                if let Some(found) = resolved(reg, module, path) {
                    out.entry(local.clone()).or_insert(found);
                }
            }
            // `pub use signal::*;` — the child's whole surface, where the head
            // of the path names a child of this module. Another crate's glob is
            // that package's business and is imported where it is used.
            (None, path) => {
                if let Some(head) = path.last().filter(|_| !path.is_empty()) {
                    if let Some(child) = def.children.get(head.as_str()) {
                        collect(reg, *child, out, seen);
                    }
                }
            }
        }
    }
}

/// The declaration a written path names, in whichever namespace holds it.
fn resolved(reg: &TypeRegistry, from: ModuleId, path: &[String]) -> Option<Def> {
    for ns in [Ns::Type, Ns::Value] {
        if let Ok(Some(def)) = reg.lookup(from, ns, path) {
            return Some(def);
        }
    }
    None
}

/// One name two star exports both offer, and what the index writes for it.
pub struct Ambiguity {
    pub name: String,
    /// The `./m` specifiers offering it, in the order the index writes them.
    pub modules: Vec<String>,
    /// The specifier that keeps the bare name, where Rust reaches one of them
    /// unqualified from the crate root.
    pub bare: Option<String>,
}

impl Ambiguity {
    /// The alias a module-qualified re-export gives this name: `broadcast`'s
    /// `ListenerGuard` is `broadcast_ListenerGuard`, so nothing is unexported.
    pub fn alias(&self, module: &str) -> String {
        let stem = module.rsplit('/').next().unwrap_or(module);
        format!("{}_{}", stem.replace('-', "_"), self.name)
    }
}

/// Which names more than one of these star exports offers, meaning DIFFERENT
/// declarations by that name.
///
/// `surfaces` is each `export * from '<specifier>'` the index is about to
/// write, with the names it contributes and the declaration each resolves to,
/// in the order the index writes them. Two modules offering one declaration —
/// core's `lineage` re-exports `retrieval`'s `GetEvents` — is not an ambiguity:
/// JavaScript resolves both star exports to the one binding. `bare_from`
/// answers, for a name, which specifier Rust itself reaches unqualified from
/// the crate root; that module keeps the bare name.
pub fn ambiguities(
    surfaces: &[(String, BTreeMap<String, Def>)],
    bare_from: impl Fn(&str) -> Option<String>,
) -> Vec<Ambiguity> {
    let mut offered: BTreeMap<&str, Vec<(&str, Def)>> = BTreeMap::new();
    for (specifier, names) in surfaces {
        for (name, def) in names {
            offered.entry(name.as_str()).or_default().push((specifier.as_str(), *def));
        }
    }
    offered
        .into_iter()
        .filter(|(_, from)| {
            let first = from[0].1;
            from.len() > 1 && from.iter().any(|(_, def)| *def != first)
        })
        .map(|(name, from)| Ambiguity {
            bare: bare_from(name).filter(|m| from.iter().any(|(spec, _)| spec == m)),
            name: name.to_string(),
            modules: from.into_iter().map(|(spec, _)| spec.to_string()).collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ty::TypeId;

    /// A declaration, spelled as a number so a test can say "the same one" and
    /// "a different one" without building a registry.
    fn decl(n: u32) -> Def { Def::Type(TypeId(n)) }

    fn amb(surfaces: &[(&str, &[(&str, u32)])], bare: Option<&str>) -> Vec<Ambiguity> {
        let owned: Vec<(String, BTreeMap<String, Def>)> = surfaces
            .iter()
            .map(|(m, ns)| {
                (m.to_string(), ns.iter().map(|(n, d)| (n.to_string(), decl(*d))).collect())
            })
            .collect();
        ambiguities(&owned, |_| bare.map(str::to_string))
    }

    #[test]
    fn a_name_only_one_module_offers_is_not_ambiguous() {
        assert!(amb(&[("./a", &[("One", 1)]), ("./b", &[("Two", 2)])], None).is_empty());
    }

    #[test]
    fn one_declaration_two_modules_re_export_is_not_ambiguous() {
        // core's `lineage.rs` writes `pub use crate::retrieval::GetEvents;`, so
        // both modules offer the name and both mean the one trait.
        assert!(amb(&[("./lineage", &[("GetEvents", 7)]), ("./retrieval", &[("GetEvents", 7)])], None).is_empty());
    }

    #[test]
    fn two_declarations_by_one_name_are_reported_with_both_modules() {
        let found = amb(&[("./a", &[("G", 1)]), ("./b", &[("G", 2)])], None);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "G");
        assert_eq!(found[0].modules, vec!["./a", "./b"]);
        // Neither is reachable unqualified from the crate root, so neither
        // takes the bare name: both are spelled with their module.
        assert!(found[0].bare.is_none());
        assert_eq!(found[0].alias("./b"), "b_G");
    }

    #[test]
    fn the_module_rust_reaches_unqualified_keeps_the_bare_name() {
        let found = amb(
            &[("./broadcast", &[("ListenerGuard", 1)]), ("./signal", &[("ListenerGuard", 2)])],
            Some("./signal"),
        );
        assert_eq!(found[0].bare.as_deref(), Some("./signal"));
        assert_eq!(found[0].alias("./broadcast"), "broadcast_ListenerGuard");
    }

    #[test]
    fn a_bare_answer_naming_a_module_that_is_not_one_of_them_is_not_used() {
        let found = amb(&[("./a", &[("G", 1)]), ("./b", &[("G", 2)])], Some("./elsewhere"));
        assert!(found[0].bare.is_none(), "a module that does not offer the name cannot keep it");
    }
}
