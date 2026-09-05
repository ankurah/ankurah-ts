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

/// What reading one sibling produced: its declarations, and the files that
/// could not be read.
pub struct Load {
    pub files: Vec<crate::registry::ExtractedFile>,
    /// One line per file the parser refused, named the way this run names it.
    /// A sibling file that does not parse is a hole in what THIS crate can
    /// resolve — every type it declared becomes a foreign name here — so the
    /// run that needs it is the run that says so.
    pub failures: Vec<String>,
}

/// Read one sibling's declarations, under that crate's own config.
///
/// The files come back named `<ident>/<path>`, which is the module path a
/// written `ankql::ast::Selection` looks up, and are marked declarations-only so
/// nothing is emitted for them.
///
/// A sibling is read under exactly the rules its own run uses: an
/// `[excluded_files]` entry is not in the port at all, an `[[excluded_items]]`
/// entry is not in the crate, and a `[[provided]]` module's members are whoever
/// wrote that TypeScript. Reading a sibling by a laxer rule than its own crate
/// gives two different answers about one crate in one registry: an excluded
/// item resolves here and nowhere else, and a provided type's methods look
/// emitted when they are hand-written.
pub fn declarations(sibling: &Sibling, config: &Config, corpus_root: &Path) -> Result<Load> {
    let features = config.features_for_crate(&sibling.cargo_name);
    let prefix = sibling
        .src
        .strip_prefix(corpus_root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| format!("{}/src", sibling.cargo_name));
    let mut out = Vec::new();
    let mut failures = Vec::new();
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
        let full_path = format!("{}/{}", prefix, relative);
        if config.is_excluded_file(&full_path) {
            continue;
        }
        let excluded_here = config.excluded_items_in(&full_path);
        let cfg = crate::extract::ExtractCfg {
            features: Some(&features),
            excluded: &excluded_here,
        };
        let named = format!("{}/{}", sibling.ident, relative);
        let mut file = match crate::extract::extract_with_cfg(entry.path(), cfg) {
            Ok(file) => file,
            Err(e) => {
                crate::extract::take_exclusions_hit();
                crate::diag::pending::discard();
                failures.push(format!("{}: {:#}", named, e));
                continue;
            }
        };
        // Every diagnostic a sibling raises belongs to that crate's own run.
        crate::extract::take_exclusions_hit();
        crate::diag::pending::discard();
        file.path = named.clone();
        out.push(crate::registry::ExtractedFile {
            path: named,
            file,
            declarations_only: true,
            // A `[[provided]]` module's TypeScript is hand-written in that
            // crate's package too, and its members are whatever the person who
            // wrote the file wrote — here as much as there.
            hand_written: config.provided_module(&full_path).is_some(),
        });
    }
    Ok(Load { files: out, failures })
}

/// Every top-level name a CALLER of this file can import: the types it declares
/// and the module-level `pub fn`s it declares.
///
/// For: the registry maps each importable name to the package that declares it,
/// and it has to answer the same for a crate being transpiled and for one read
/// as a dependency — a name that resolves in one run and not the other is the
/// equivalence gap. The sibling path listed types and traits and stopped there,
/// so a call across a crate boundary emitted a bare unresolved name: 14
/// `parseSelection` sites, `parseSelection is not defined` in every one.
pub fn importable_names(file: &crate::types::RustFile) -> Vec<String> {
    file.structs
        .iter()
        .map(|s| s.name.clone())
        .chain(file.enums.iter().map(|e| e.name.clone()))
        .chain(file.traits.iter().map(|t| t.name.clone()))
        .chain(file.functions.iter().filter(|f| f.is_pub).map(|f| f.ts_name.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> (Config, BTreeMap<String, PathBuf>, PathBuf) {
        let config = Config::load(Path::new("transpile.toml")).unwrap();
        let root = std::fs::canonicalize(&config.paths.rust_source)
            .unwrap_or_else(|_| config.paths.rust_source.clone());
        let located = locate(&config);
        (config, located, root)
    }

    fn ankql_as_a_sibling() -> (Config, Load) {
        let (config, located, root) = corpus();
        // ankurah-core depends on ankql, so this is how core's run reads it.
        let sibling = dependencies_of(&config, &located, "ankurah-core")
            .into_iter()
            .find(|s| s.cargo_name == "ankql")
            .expect("ankurah-core depends on ankql");
        let load = declarations(&sibling, &config, &root).unwrap();
        (config, load)
    }

    /// A crate read as a dependency is the same crate it is when it is the one
    /// being transpiled: same files, same items, same hand-written members.
    /// Two answers about one crate in one registry is a name that resolves here
    /// and nowhere else.
    #[test]
    fn a_sibling_is_read_under_its_own_crate_s_rules() {
        let (config, load) = ankql_as_a_sibling();
        assert!(!load.files.is_empty(), "ankql has sources");
        assert!(
            load.failures.is_empty(),
            "the corpus parses today; these did not: {:?}",
            load.failures
        );

        // `[[provided]]`: `ankql/src/grammar.rs` is a pest grammar whose
        // TypeScript somebody wrote, so its members are hand-written here too.
        let grammar = load
            .files
            .iter()
            .find(|f| f.path.ends_with("grammar.rs"))
            .expect("ankql declares grammar.rs");
        assert!(grammar.hand_written, "a `[[provided]]` module's members are hand-written");
        assert!(grammar.declarations_only, "nothing is emitted for a sibling");
        let ordinary = load
            .files
            .iter()
            .find(|f| f.path.ends_with("ast.rs"))
            .expect("ankql declares ast.rs");
        assert!(!ordinary.hand_written, "an ordinary module is emitted by its own crate's run");

        // `[[excluded_items]]`: `impl From<ParseError> for JsValue` is an error
        // crossing the wasm ABI and is not in the port. Read without the
        // exclusion it is a declaration here and in no other run of ankql.
        let error_rs = load
            .files
            .iter()
            .find(|f| f.path.ends_with("error.rs"))
            .expect("ankql declares error.rs");
        assert!(
            !error_rs.file.impls.iter().any(|i| i.target_type.contains("JsValue")),
            "the excluded impl is out of the crate, however the crate is read"
        );
        // The exclusion is real: the config still names it, so a future config
        // that drops the entry moves this test rather than passing quietly.
        assert!(
            config
                .excluded_items_in("ankql/src/error.rs")
                .iter()
                .any(|e| e.written.contains("JsValue")),
            "the config excludes that impl"
        );
    }

    /// PREMISE ADDED 2026-09-05 (fixpass4 item 6): every KIND of top-level name
    /// a caller imports has to come back from a sibling, not only the types.
    /// A module-level `pub fn` is a name a caller imports exactly as a type is,
    /// and it was the one kind the sibling path left out.
    #[test]
    fn a_siblings_importable_names_cover_every_kind_a_caller_imports() {
        let (_, load) = ankql_as_a_sibling();
        let names: Vec<String> =
            load.files.iter().flat_map(|f| importable_names(&f.file)).collect();
        // a struct, an enum, a trait-free module-level function
        assert!(names.iter().any(|n| n == "Selection"), "a struct: {:?}", names);
        assert!(names.iter().any(|n| n == "Predicate"), "an enum: {:?}", names);
        assert!(
            names.iter().any(|n| n == "parseSelection"),
            "a module-level `pub fn`, in the TypeScript spelling a caller writes: {:?}",
            names
        );
    }

    /// `[excluded_files]` is a file the port does not have at all.
    #[test]
    fn a_sibling_does_not_carry_a_file_the_port_excludes() {
        let (config, load) = ankql_as_a_sibling();
        for file in &load.files {
            let relative = file.path.splitn(2, '/').nth(1).unwrap_or(&file.path);
            assert!(
                !config.is_excluded_file(&format!("ankql/src/{}", relative)),
                "{} is excluded and was read as a sibling anyway",
                file.path
            );
        }
    }
}
