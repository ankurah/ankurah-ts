//! What a module RE-EXPORTS, and the names a `pub use *` has to disambiguate.
//!
//! Split out of `codegen.rs`, which was over the 600-line rule and grew again
//! when the enum-variant `collect_type_refs` squash was undone (R10: the ratchet
//! is met by splitting, never by joining lines). Rust's `pub mod` and `pub use`
//! say what a crate offers under its own name; TypeScript has no such thing, so
//! the port writes the re-exports out — and two `export *` that offer the same
//! name are an error there where they are not one in Rust.

use crate::registry::TypeRegistry;
use crate::types::RustFile;


/// What this file re-exports from the modules under it, as the `export` lines
/// the port writes.
///
/// `pub use auth::*;` and `pub use subscription::QueryId;` are what make a
/// crate's names reachable from its root, and the port's `index.ts` has to say
/// the same or the package offers nothing. Without them the emitted index was a
/// header and a blank line, and `QueryId` — re-exported by name — was reachable
/// only by importing the module it was declared in.
///
/// Only a module this file declares: `pub use serde::*` is another crate's
/// business, and the cross-crate import machinery writes that where it is used.
pub(super) fn public_reexports(
    reg: &TypeRegistry,
    file: &RustFile,
    corpus_path: &str,
    config: Option<&crate::config::Config>,
) -> Vec<String> {
    let Some(module) = reg.modules().lookup_file(&file.path) else {
        return Vec::new();
    };
    let children = &reg.modules().get(module).children;
    let mut out: Vec<String> = Vec::new();
    // `pub mod ast;` is how `ankql::ast::Expr` becomes reachable from outside
    // the crate. TypeScript has no nested module namespace to mirror, and the
    // port's own hand-written indexes settled the convention long ago: a public
    // child module is re-exported whole. Without this the emitted `index.ts`
    // for a crate whose root is nothing but `pub mod` lines — ankql's — was a
    // header and a blank line, and the package exported nothing at all.
    let mut whole_modules: Vec<String> = Vec::new();
    // Which module each `export * from './m'` flattens, so the ambiguity pass
    // below can ask what names it brings. A `[[provided]]` module is somebody's
    // hand-written TypeScript and the registry does not hold its names, so it
    // is left out of that question.
    let mut star_modules: Vec<(String, crate::registry::ModuleId)> = Vec::new();
    for (name, vis) in &file.mod_decls {
        if *vis != crate::types::VisInfo::Public {
            continue;
        }
        let provided = provided_child_module(corpus_path, name, config);
        let target = provided.clone().unwrap_or_else(|| child_module(&file.path, name));
        let line = format!("export * from '{}';\n", target);
        if !out.contains(&line) {
            out.push(line);
            if provided.is_none() {
                if let Some(child) = children.get(name) {
                    star_modules.push((target.clone(), *child));
                }
            }
            whole_modules.push(target);
        }
    }
    // A TypeScript-only module the port adds beside this crate.
    if let Some(cfg) = config {
        for extra in cfg.extra_exports_in(corpus_path) {
            let line = format!("export * from './{}';\n", extra.module);
            if !out.contains(&line) {
                out.push(line);
            }
        }
    }
    // `pub use ankurah_proto as proto;` gives the crate a LOCAL name, and the
    // line below it — `pub use proto::EntityId;` — reaches the crate through
    // that name. Without the map the second line names nothing and `EntityId`
    // was simply absent from the facade's surface.
    let mut crate_aliases: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for u in &file.uses {
        for binding in &u.bindings {
            if let (Some(local), [one]) = (&binding.local, &binding.path[..]) {
                if reg.sibling_crate(one).is_some() {
                    crate_aliases.insert(local.as_str(), one.as_str());
                }
            }
        }
    }
    for u in &file.uses {
        if u.vis != crate::types::VisInfo::Public {
            continue;
        }
        for binding in &u.bindings {
            // `pub use ankurah_proto as proto;` and `pub use proto::EntityId;`
            // name ANOTHER CRATE, and a crate is a package here, not a file
            // beside this one. The registry keeps a sibling's root among this
            // module's children, so asking `children` alone wrote
            // `export { proto } from './ankurah_proto'` — 29 broken module
            // specifiers in the facade's index, which is every own-file error
            // that package had.
            let head = binding.path.first().map(String::as_str).unwrap_or_default();
            let head = crate_aliases.get(head).copied().unwrap_or(head);
            if let Some(package) = sibling_package(reg, head) {
                // `pub use ankurah_core::{changes, entity, ..}` re-exports
                // another crate's MODULES. The port flattens a crate's modules
                // into its package surface — `export * from './changes'` — so
                // there is no `changes` name on the other side to re-export,
                // and writing one names nothing.
                if let (Some(local), [_, name]) = (&binding.local, &binding.path[..]) {
                    if reg
                        .sibling_crate(head)
                        .is_some_and(|root| reg.modules().get(root).children.contains_key(name))
                    {
                        crate::diag::pending::park_at(
                            0,
                            0,
                            format!(
                                "`{}` re-exports `{}`, which is a MODULE of that crate, and the \
                                 port flattens a crate's modules into its package surface, so \
                                 there is no name to re-export",
                                package, local
                            ),
                        );
                        continue;
                    }
                }
                let line = match (&binding.local, &binding.path[..]) {
                    // `pub use ankql;` / `pub use ankurah_core as core;` —
                    // the whole crate under one name.
                    (Some(local), [_one]) => {
                        format!("export * as {} from '{}';\n", local, package)
                    }
                    // `pub use proto::EntityId;` — one name out of it.
                    (Some(local), [_, ..]) => {
                        format!("export {{ {} }} from '{}';\n", local, package)
                    }
                    // `pub use ankurah_derive::*;`
                    (None, _) => format!("export * from '{}';\n", package),
                    // A binding with a local name and no path is not a shape
                    // `use` produces.
                    (Some(_), []) => continue,
                };
                if !out.contains(&line) {
                    out.push(line);
                }
                continue;
            }
            let line = match (&binding.local, &binding.path[..]) {
                (None, [name]) if children.contains_key(name) => {
                    let provided = provided_child_module(corpus_path, name, config);
                    let target = provided.clone().unwrap_or_else(|| child_module(&file.path, name));
                    if provided.is_none() && !star_modules.iter().any(|(t, _)| *t == target) {
                        star_modules.push((target.clone(), children[name]));
                    }
                    format!("export * from '{}';\n", target)
                }
                (Some(local), [name, ..]) if children.contains_key(name) => {
                    let target = provided_child_module(corpus_path, name, config)
                        .unwrap_or_else(|| child_module(&file.path, name));
                    // `pub mod broadcast;` beside `pub use broadcast::BroadcastId;`
                    // is two true statements about one module, and the star
                    // export already carries the name.
                    if whole_modules.contains(&target) {
                        continue;
                    }
                    format!("export {{ {} }} from '{}';\n", local, target)
                }
                _ => continue,
            };
            if !out.contains(&line) {
                out.push(line);
            }
        }
    }
    out.extend(disambiguate_stars(reg, file, &star_modules));
    out
}

/// The explicit re-exports that keep a name two star exports both offer.
///
/// `export * from './broadcast'` and `export * from './signal'` both offering
/// `ListenerGuard` means JavaScript exports it from NEITHER — an ambiguous star
/// export is dropped silently, so `@ankurah/signals` had no `ListenerGuard` at
/// all in either spelling. An explicit export shadows every star export of that
/// name, so writing one settles it: the module Rust itself reaches unqualified
/// from the crate root (`pub use signal::*`) keeps the bare name, and every
/// other module keeps its own under a module qualifier. Where Rust reaches
/// none of them unqualified there is no bare name to award, so all of them are
/// qualified and the report says the bare spelling is gone.
pub(super) fn disambiguate_stars(
    reg: &TypeRegistry,
    file: &RustFile,
    star_modules: &[(String, crate::registry::ModuleId)],
) -> Vec<String> {
    use crate::codegen::surface;
    if star_modules.len() < 2 {
        return Vec::new();
    }
    let surfaces: Vec<(String, std::collections::BTreeMap<String, crate::registry::Def>)> = star_modules
        .iter()
        .map(|(specifier, id)| (specifier.clone(), surface::star_surface(reg, *id)))
        .collect();

    // Which specifier, if any, Rust reaches this name through UNQUALIFIED from
    // the crate root. `pub mod broadcast;` is not such a reach: it makes the
    // name reachable only as `broadcast::ListenerGuard`.
    let mut unqualified: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for u in &file.uses {
        if u.vis != crate::types::VisInfo::Public {
            continue;
        }
        for binding in &u.bindings {
            let Some(head) = binding.path.first() else { continue };
            let Some((specifier, _)) = star_modules
                .iter()
                .find(|(t, _)| t.rsplit('/').next() == Some(head.as_str()))
            else {
                continue;
            };
            match (&binding.local, &binding.path[..]) {
                // `pub use signal::ListenerGuard;` — this one name.
                (Some(local), [_, ..]) => {
                    unqualified.insert(local.clone(), specifier.clone());
                }
                // `pub use signal::*;` — everything the module offers.
                (None, [_]) => {
                    for name in surface::star_surface(reg, star_modules.iter().find(|(t, _)| t == specifier).unwrap().1).into_keys() {
                        unqualified.entry(name).or_insert_with(|| specifier.clone());
                    }
                }
                _ => {}
            }
        }
    }

    let mut lines = Vec::new();
    for found in surface::ambiguities(&surfaces, |name| unqualified.get(name).cloned()) {
        for specifier in &found.modules {
            if found.bare.as_deref() == Some(specifier.as_str()) {
                lines.push(format!("export {{ {} }} from '{}';\n", found.name, specifier));
            } else {
                lines.push(format!(
                    "export {{ {} as {} }} from '{}';\n",
                    found.name,
                    found.alias(specifier),
                    specifier
                ));
            }
        }
        let where_bare = match &found.bare {
            Some(m) => format!("`{}` keeps the bare name because the crate root reaches it there unqualified", m),
            None => format!(
                "the crate root reaches none of them unqualified, so `{}` is not exported bare at all",
                found.name
            ),
        };
        crate::diag::pending::park_at(
            0,
            0,
            format!(
                "`{}` is declared in {}, and the port flattens a crate's modules into one package \
                 surface, where two star exports of one name export it from neither. Each keeps \
                 its own name qualified by its module ({}); {}",
                found.name,
                found.modules.join(" and "),
                found
                    .modules
                    .iter()
                    .filter(|m| found.bare.as_deref() != Some(m.as_str()))
                    .map(|m| found.alias(m))
                    .collect::<Vec<_>>()
                    .join(", "),
                where_bare
            ),
        );
    }
    lines
}

/// The package a `pub use` of another crate re-exports from, where the head of
/// the path names one.
///
/// A crate the port does not carry — `ankurah_derive`, whose macros are
/// expanded away — has no package to name, and the re-export is reported rather
/// than written against a specifier nothing resolves.
pub(super) fn sibling_package(reg: &TypeRegistry, head: &str) -> Option<String> {
    reg.sibling_crate(head)?;
    match crate::name_map::map_crate_to_package(head) {
        Some(package) => Some(package.to_string()),
        None => None,
    }
}

/// Where a hand-written child module sits, when the TypeScript it is called is
/// not what the Rust module is called. A `[[provided]]` entry names both, so a
/// re-export of `mod connection;` reaches `connection.provided.ts` where that is
/// what somebody wrote.
pub(super) fn provided_child_module(
    corpus_path: &str,
    child: &str,
    config: Option<&crate::config::Config>,
) -> Option<String> {
    let cfg = config?;
    // The parent's own directory, as a corpus path: `ankql/src/lib.rs` puts its
    // children at `ankql/src/<child>.rs`.
    let dir = corpus_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let stem = corpus_path
        .rsplit('/')
        .next()
        .unwrap_or(corpus_path)
        .trim_end_matches(".rs");
    let candidate = match (dir, stem) {
        ("", "lib") | ("", "mod") => format!("{child}.rs"),
        ("", other) => format!("{other}/{child}.rs"),
        (dir, "lib") | (dir, "mod") => format!("{dir}/{child}.rs"),
        (dir, other) => format!("{dir}/{other}/{child}.rs"),
    };
    let provided = cfg.provided_module(&candidate)?;
    // `module` is relative to the package's src/; this file imports it relative
    // to itself, and everything that re-exports a child is a module index.
    let last = provided.module.rsplit('/').next().unwrap_or(&provided.module);
    Some(match (dir, stem) {
        (_, "lib") | (_, "mod") => format!("./{last}"),
        (_, other) => format!("./{other}/{last}"),
    })
}

/// Where a child module's file sits, as this file would import it.
///
/// A crate root — `lib.rs`, emitted as `index.ts` — has its children beside it:
/// `./auth`. Any other module keeps its children in a directory named after
/// itself, so `signal.rs`'s `calculated` is at `./signal/calculated`. Writing
/// `./calculated` from `signal.ts` named a file that is not there.
pub(super) fn child_module(file_path: &str, child: &str) -> String {
    let stem = file_path
        .rsplit('/')
        .next()
        .unwrap_or(file_path)
        .trim_end_matches(".rs");
    match stem {
        "lib" | "mod" => format!("./{}", child),
        other => format!("./{}/{}", other, child),
    }
}

