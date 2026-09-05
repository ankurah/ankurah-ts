//! The other crates of the port, loaded for their declarations.
//!
//! `ankurah_proto::request::NodeRequestBody` holds an `ankql::ast::Selection`.
//! Transpiling proto on its own left `Selection` as a foreign name with no
//! declaration and no import, and the emitted decoder said `Selection is not
//! defined` at run time. A hand-written `[cross_crate_types]` entry papered over
//! the one case somebody noticed.
//!
//! So when a crate is transpiled, every in-family crate it depends on is read
//! too — its DECLARATIONS only, never its bodies, and never emitted. Their types
//! land in the same registry as this crate's own, under a module named for the
//! crate, so `ankql::ast::Selection` resolves to a real type with a real id
//! rather than to a foreign one; and the import map sends them to
//! `@ankurah/<package>`, which is where the port writes them.
//!
//! The dependency edges come from each crate's own Cargo.toml, filtered by
//! `[crates]` — a dependency outside the port's scope is not loaded, because
//! there is nothing on the other side of that edge here.

use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::config::Config;

/// One in-family crate, as this run needs it.
pub struct Sibling {
    /// The Rust identifier a path names it by: `ankurah_proto`, `ankql`.
    pub ident: String,
    /// Its TypeScript package: `@ankurah/proto`.
    pub package: String,
    /// Its `src` directory in the corpus.
    pub src: PathBuf,
    /// The Cargo name, for the feature set.
    pub cargo_name: String,
}

/// Where each in-scope crate lives in the corpus, by Cargo name.
///
/// The `[crates]` table says which crates the port has and what each becomes;
/// it does not say where they sit, because that is the corpus's business. One
/// walk of the corpus reads it off the `Cargo.toml` files themselves, so a crate
/// that moves needs no config change.
pub fn locate(config: &Config) -> BTreeMap<String, PathBuf> {
    let mut found = BTreeMap::new();
    let root = std::fs::canonicalize(&config.paths.rust_source)
        .unwrap_or_else(|_| config.paths.rust_source.clone());
    for entry in walkdir::WalkDir::new(&root)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "Cargo.toml")
    {
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        let Ok(table) = text.parse::<toml::Table>() else {
            continue;
        };
        let Some(name) = table
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
        else {
            continue;
        };
        if !config.crates.contains_key(name) {
            continue;
        }
        if let Some(dir) = entry.path().parent() {
            found.insert(name.to_string(), dir.to_path_buf());
        }
    }
    found
}

/// The in-family crates `cargo_name` depends on, transitively, minus itself.
pub fn dependencies_of(
    config: &Config,
    located: &BTreeMap<String, PathBuf>,
    cargo_name: &str,
) -> Vec<Sibling> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut queue = vec![cargo_name.to_string()];
    while let Some(next) = queue.pop() {
        for dep in direct_dependencies(located.get(&next)) {
            if !config.crates.contains_key(&dep) || dep == cargo_name {
                continue;
            }
            if seen.insert(dep.clone()) {
                queue.push(dep);
            }
        }
    }
    seen.into_iter()
        .filter_map(|name| {
            let dir = located.get(&name)?;
            Some(Sibling {
                ident: name.replace('-', "_"),
                package: format!("@ankurah/{}", config.crates.get(&name)?),
                src: dir.join("src"),
                cargo_name: name,
            })
        })
        .collect()
}

/// The dependency names one Cargo.toml declares, dev-dependencies included: a
/// test in this crate reaches them too.
fn direct_dependencies(dir: Option<&PathBuf>) -> Vec<String> {
    let Some(dir) = dir else { return Vec::new() };
    let Ok(text) = std::fs::read_to_string(dir.join("Cargo.toml")) else {
        return Vec::new();
    };
    let Ok(table) = text.parse::<toml::Table>() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for section in ["dependencies", "dev-dependencies"] {
        if let Some(deps) = table.get(section).and_then(|d| d.as_table()) {
            out.extend(deps.keys().cloned());
        }
    }
    out
}

/// Read one sibling's declarations. The files come back named `<ident>/<path>`,
/// which is the module path a written `ankql::ast::Selection` looks up, and are
/// marked declarations-only so nothing is emitted for them.
pub fn declarations(
    sibling: &Sibling,
    config: &Config,
    corpus_root: &Path,
) -> Result<Vec<crate::registry::ExtractedFile>> {
    let features = config.features_for_crate(&sibling.cargo_name);
    let prefix = sibling
        .src
        .strip_prefix(corpus_root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| format!("{}/src", sibling.cargo_name));
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(&sibling.src)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
    {
        let relative = entry
            .path()
            .strip_prefix(&sibling.src)
            .unwrap_or(entry.path())
            .display()
            .to_string();
        if config.is_excluded_file(&format!("{}/{}", prefix, relative)) {
            continue;
        }
        let cfg = crate::extract::ExtractCfg {
            features: Some(&features),
            excluded: &[],
        };
        let Ok(mut file) = crate::extract::extract_with_cfg(entry.path(), cfg) else {
            continue;
        };
        // Every diagnostic a sibling raises belongs to that crate's own run.
        crate::extract::take_exclusions_hit();
        crate::diag::pending::discard();
        file.path = format!("{}/{}", sibling.ident, relative);
        out.push(crate::registry::ExtractedFile {
            path: file.path.clone(),
            file,
            declarations_only: true,
            hand_written: false,
        });
    }
    Ok(out)
}
