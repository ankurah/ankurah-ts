//! Configuration parsing — reads transpile.toml
//!
//! The file answers four questions the engine cannot work out for itself: which
//! crates the port has (`[crates]`), what build it is being transpiled for
//! (`[cfg]`, `[features.*]`, `[[feature_overrides]]`), what is deliberately not
//! in the port (`[excluded_files]`, `[[excluded_items]]`), and which modules are
//! written by hand (`[[provided]]`, `[provided_impls]`).

use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

mod items;
pub use items::ItemSelector;

#[derive(Debug)]
pub struct Config {
    pub paths: PathsConfig,
    /// Cargo crate name → TypeScript package name. The port's whole scope.
    pub crates: HashMap<String, String>,
    pub excluded_files: Vec<String>,
    /// Items excluded one at a time, with the reason each is out.
    pub excluded_items: Vec<ExcludedItem>,
    /// Modules whose TypeScript is written by hand.
    pub provided_modules: Vec<ProvidedModule>,
    /// TypeScript-only modules a module index re-exports.
    pub extra_exports: Vec<ProvidedModule>,
    pub provided_impls: HashMap<String, ProvidedImpl>,
    /// Types from other crates that need explicit import mapping
    pub cross_crate_types: HashMap<String, String>,
    /// Target and profile predicates, the same for every crate in one build.
    cfg_key_values: BTreeMap<String, String>,
    cfg_flags: BTreeMap<String, bool>,
    /// Cargo's resolved feature set per Cargo crate name.
    crate_features: HashMap<String, Vec<String>>,
    /// The recorded departures from Cargo's resolved set.
    pub feature_overrides: Vec<FeatureOverride>,
    /// Every feature each crate's own `Cargo.toml` DECLARES, implicit ones
    /// included. Filled from the corpus at load; empty when the corpus is not
    /// where `[paths] rust_source` says, which is what a unit fixture looks
    /// like. A crate with an entry here can say "nothing decides this feature"
    /// instead of answering false to a name nobody declared.
    declared_features: HashMap<String, Vec<String>>,
}

#[derive(Debug)]
pub struct PathsConfig {
    pub rust_source: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ProvidedImpl {
    pub path: String,
    /// Does the hand-written file declare `static fromJson` for this type?
    ///
    /// The engine never reads the TypeScript it did not write, so a provided
    /// type's members are whatever the person who wrote the file wrote. Reading
    /// "it is hand-written" as "it reads JSON" put `Attested.fromJson` in three
    /// emitted call sites where `auth.provided.ts` declares no such static.
    /// Each entry says which it is, and a type that does not is refused —
    /// transitively, so nothing holding one gets a JSON half either.
    pub reads_json: bool,
}

/// One item the port leaves out, named the way the corpus writes it.
#[derive(Debug, Clone)]
pub struct ExcludedItem {
    /// Corpus-relative path, e.g. `core/src/model.rs`.
    pub file: String,
    pub selector: ItemSelector,
    pub written: String,
    /// Why this item is out. Required at load — an exclusion with no reason is
    /// a config error — and printed beside the `EXCLUDED` line the run writes
    /// when the item is actually found and dropped.
    pub reason: String,
}

/// One module whose TypeScript is hand-written.
#[derive(Debug, Clone)]
pub struct ProvidedModule {
    /// Corpus-relative path, e.g. `ankql/src/parser.rs`.
    pub file: String,
    /// The TypeScript module it is, relative to the package's `src/`, without
    /// the extension — what the module index re-exports from.
    pub module: String,
    pub reason: String,
}

/// A feature Cargo resolves one way and the port takes the other, with the
/// reason. The only sanctioned class is a feature pulling an out-of-family
/// framework into the port.
#[derive(Debug, Clone)]
pub struct FeatureOverride {
    pub krate: String,
    pub feature: String,
    /// `true` = forced on, `false` = forced off.
    pub state: bool,
    /// Why the port departs from Cargo's resolved set here. Required at load,
    /// and said in the `FEATURE` line the run prints for each override — the
    /// one class of deliberate departure ought to be visible in the output that
    /// records what the build decided.
    pub reason: String,
}

impl Config {
    /// Is this fully qualified Rust type's TypeScript written by hand?
    ///
    /// Read by this module's own tests, which pin which of proto's types the
    /// port provides. Emission asks the registry instead — `is_hand_written`,
    /// which the loader sets from this table.
    #[cfg(test)]
    pub fn is_provided(&self, rust_fqn: &str) -> bool {
        self.provided_impls.contains_key(rust_fqn)
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;

        let table: toml::Table = content
            .parse()
            .with_context(|| "Failed to parse transpile.toml")?;

        if table.contains_key("hardcode") {
            bail!(
                "transpile.toml still has a [hardcode] table. It was replaced by [[provided]], \
                 which also names the TypeScript module the hand-written file is, so the \
                 module index can re-export it."
            );
        }

        let paths = if let Some(paths) = table.get("paths").and_then(|v| v.as_table()) {
            PathsConfig {
                rust_source: locate_corpus(
                    paths
                        .get("rust_source")
                        .and_then(|v| v.as_str())
                        .unwrap_or("../ankurah-ts-support"),
                ),
            }
        } else {
            PathsConfig {
                rust_source: locate_corpus("../ankurah-ts-support"),
            }
        };

        let crates = parse_string_map(table.get("crates"));

        let (cfg_key_values, cfg_flags) = parse_cfg_table(table.get("cfg"));
        let crate_features = parse_crate_features(table.get("features"))?;
        let feature_overrides = parse_feature_overrides(table.get("feature_overrides"))?;
        for over in &feature_overrides {
            if !crates.contains_key(&over.krate) {
                bail!(
                    "[[feature_overrides]] names crate `{}`, which is not in [crates]",
                    over.krate
                );
            }
        }

        let excluded_files = string_list(table.get("excluded_files"), "files");
        let excluded_items = parse_excluded_items(table.get("excluded_items"))?;
        let provided_modules = parse_provided_modules(table.get("provided"))?;
        let extra_exports = parse_provided_modules(table.get("extra_exports"))?;
        let provided_impls = parse_provided_impls(table.get("provided_impls"));
        let cross_crate_types = parse_string_map(table.get("cross_crate_types"));

        // What each crate's own Cargo.toml declares. A feature the config names
        // and the crate does not is a config that has gone stale against the
        // corpus, and a `#[cfg(feature = "x")]` naming an undeclared feature is
        // a question nothing answers rather than a false.
        let declared_features = declared_features(&crates, &paths.rust_source);
        if !declared_features.is_empty() {
            let mut stale: Vec<String> = Vec::new();
            for (krate, named) in crate_features.iter() {
                let Some(declared) = declared_features.get(krate) else { continue };
                for feature in named {
                    if !declared.contains(feature) {
                        stale.push(format!("[features.{}] names `{}`", krate, feature));
                    }
                }
            }
            for over in &feature_overrides {
                let Some(declared) = declared_features.get(&over.krate) else { continue };
                if !declared.contains(&over.feature) {
                    stale.push(format!(
                        "[[feature_overrides]] names `{}` for `{}`",
                        over.feature, over.krate
                    ));
                }
            }
            if !stale.is_empty() {
                bail!(
                    "the config has gone stale against the corpus: {} — no such feature is \
                     declared in that crate's Cargo.toml",
                    stale.join("; ")
                );
            }
        }

        Ok(Config {
            paths,
            crates,
            excluded_files,
            excluded_items,
            provided_modules,
            extra_exports,
            provided_impls,
            cross_crate_types,
            cfg_key_values,
            cfg_flags,
            crate_features,
            feature_overrides,
            declared_features,
        })
    }

    /// The cfg configuration for one Cargo crate: its resolved feature set with
    /// the recorded overrides applied, plus the build's target and profile
    /// predicates.
    pub fn features_for_crate(&self, cargo_crate: &str) -> crate::cfg::CfgFeatures {
        let declared = self.declared_features.get(cargo_crate).cloned();
        let mut enabled: Vec<String> = self
            .crate_features
            .get(cargo_crate)
            .cloned()
            .unwrap_or_default();
        for over in &self.feature_overrides {
            if over.krate != cargo_crate {
                continue;
            }
            eprintln!(
                "  FEATURE {} {} = {} ({})",
                over.krate,
                over.feature,
                over.state,
                over.reason.lines().next().unwrap_or_default()
            );
            if over.state {
                if !enabled.iter().any(|f| f == &over.feature) {
                    enabled.push(over.feature.clone());
                }
            } else {
                enabled.retain(|f| f != &over.feature);
            }
        }
        let features = crate::cfg::CfgFeatures::new(enabled)
            .with_key_values(self.cfg_key_values.clone())
            .with_flags(self.cfg_flags.clone());
        match declared {
            Some(declared) => features.with_declared(declared),
            None => features,
        }
    }

    /// The cfg configuration for the TypeScript package name `batch` is given.
    pub fn features_for_package(&self, package: &str) -> crate::cfg::CfgFeatures {
        match self.cargo_crate_for_package(package) {
            Some(krate) => self.features_for_crate(&krate),
            None => crate::cfg::CfgFeatures::new(Vec::new())
                .with_key_values(self.cfg_key_values.clone())
                .with_flags(self.cfg_flags.clone()),
        }
    }

    /// The Cargo crate a TypeScript package name comes from.
    pub fn cargo_crate_for_package(&self, package: &str) -> Option<String> {
        self.crates
            .iter()
            .find(|(_, pkg)| pkg.as_str() == package)
            .map(|(krate, _)| krate.clone())
    }

    /// Is this TypeScript package name in the port's scope at all? `batch`
    /// refuses a crate that is not: a silent skip would drop a whole crate.
    pub fn is_in_scope(&self, package: &str) -> bool {
        self.crates.values().any(|pkg| pkg == package)
    }

    pub fn packages_in_scope(&self) -> Vec<String> {
        let mut names: Vec<String> = self.crates.values().cloned().collect();
        names.sort();
        names
    }



    /// Check if a file should be excluded
    pub fn is_excluded_file(&self, path: &str) -> bool {
        self.excluded_files.iter().any(|f| path.ends_with(f))
    }

    /// The `[[provided]]` entry for a corpus-relative file path, if it has one.
    pub fn provided_module(&self, path: &str) -> Option<&ProvidedModule> {
        self.provided_modules.iter().find(|p| path.ends_with(&p.file))
    }

    /// The TypeScript-only modules this file's index re-exports.
    pub fn extra_exports_in(&self, path: &str) -> Vec<&ProvidedModule> {
        self.extra_exports
            .iter()
            .filter(|e| path.ends_with(&e.file))
            .collect()
    }

    /// Is this file's TypeScript hand-written? (The old `[hardcode]` question.)
    /// Read by this module's own tests. `[hardcode]` is a load error now and
    /// `[[provided]]` is what a hand-written module is written as; this answers
    /// the same question of that table.
    #[cfg(test)]
    pub fn is_hardcoded(&self, path: &str) -> bool {
        self.provided_module(path).is_some()
    }

    /// The `[[excluded_items]]` entries that name items in this file.
    pub fn excluded_items_in(&self, path: &str) -> Vec<&ExcludedItem> {
        self.excluded_items
            .iter()
            .filter(|e| path.ends_with(&e.file))
            .collect()
    }

    /// Get the import module path for a provided type (e.g., "./id.provided")
    pub fn provided_import_module(&self, rust_fqn: &str) -> Option<String> {
        self.provided_impls
            .get(rust_fqn)
            .map(|p| format!("./{}", p.path))
    }

}

/// Where the Rust corpus is.
///
/// `[paths] rust_source` is written relative to the transpiler's directory, and
/// it is right in the main checkout and wrong in every git worktree — a worktree
/// sits two directories deeper, so `../ankurah-ts-support` names nothing. Every
/// path in this file is then matched against a fallback spelling of the crate's
/// directory, and a `[[provided]]` or `[[excluded_items]]` entry for a crate
/// whose directory does not match its package name silently matches nothing.
/// So the configured path is used when it exists, and otherwise a directory of
/// that name is looked for above this one, which finds the corpus from the main
/// checkout and from a worktree alike.
fn locate_corpus(configured: &str) -> PathBuf {
    let configured = PathBuf::from(configured);
    if configured.join("proto/src").is_dir() {
        return configured;
    }
    let name = configured
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_else(|| std::ffi::OsString::from("ankurah-ts-support"));
    let here = std::env::current_dir().unwrap_or_default();
    for ancestor in here.ancestors() {
        let candidate = ancestor.join(&name);
        if candidate.join("proto/src").is_dir() {
            return candidate;
        }
    }
    configured
}

fn string_list(value: Option<&toml::Value>, key: &str) -> Vec<String> {
    value
        .and_then(|v| v.as_table())
        .and_then(|t| t.get(key))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_string_map(value: Option<&toml::Value>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(table) = value.and_then(|v| v.as_table()) {
        for (k, v) in table {
            if let Some(s) = v.as_str() {
                map.insert(k.clone(), s.to_string());
            }
        }
    }
    map
}

/// `[cfg]`: string values are name-value predicates, booleans are bare flags.
fn parse_cfg_table(
    value: Option<&toml::Value>,
) -> (BTreeMap<String, String>, BTreeMap<String, bool>) {
    let mut kvs = BTreeMap::new();
    let mut flags = BTreeMap::new();
    if let Some(table) = value.and_then(|v| v.as_table()) {
        for (k, v) in table {
            match v {
                toml::Value::String(s) => {
                    kvs.insert(k.clone(), s.clone());
                }
                toml::Value::Boolean(b) => {
                    flags.insert(k.clone(), *b);
                }
                _ => {}
            }
        }
    }
    (kvs, flags)
}

/// `[features.<crate>] enabled = [..]`, with the old flat
/// `[features] enabled = [..]` still read as the fallback for every crate.
fn parse_crate_features(value: Option<&toml::Value>) -> Result<HashMap<String, Vec<String>>> {
    let mut per_crate = HashMap::new();
    let Some(table) = value.and_then(|v| v.as_table()) else {
        return Ok(per_crate);
    };
    for (key, v) in table {
        match v {
            toml::Value::Table(inner) => {
                let enabled = inner
                    .get("enabled")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                per_crate.insert(key.clone(), enabled);
            }
            toml::Value::Array(_) if key == "enabled" => {
                bail!(
                    "transpile.toml has a crate-wide `[features] enabled`. Features are \
                     Cargo's resolved set PER CRATE now: write `[features.<cargo-crate>]`."
                );
            }
            _ => {}
        }
    }
    Ok(per_crate)
}

fn parse_feature_overrides(value: Option<&toml::Value>) -> Result<Vec<FeatureOverride>> {
    let mut out = Vec::new();
    let Some(array) = value.and_then(|v| v.as_array()) else {
        return Ok(out);
    };
    for entry in array {
        let t = entry
            .as_table()
            .context("[[feature_overrides]] entries are tables")?;
        let krate = required_str(t, "crate", "feature_overrides")?;
        let feature = required_str(t, "feature", "feature_overrides")?;
        let state = match required_str(t, "state", "feature_overrides")?.as_str() {
            "on" => true,
            "off" => false,
            other => bail!("[[feature_overrides]] state is `on` or `off`, not `{other}`"),
        };
        let reason = required_str(t, "reason", "feature_overrides")?;
        out.push(FeatureOverride {
            krate,
            feature,
            state,
            reason,
        });
    }
    Ok(out)
}

fn parse_excluded_items(value: Option<&toml::Value>) -> Result<Vec<ExcludedItem>> {
    let mut out = Vec::new();
    let Some(array) = value.and_then(|v| v.as_array()) else {
        return Ok(out);
    };
    for entry in array {
        let t = entry
            .as_table()
            .context("[[excluded_items]] entries are tables")?;
        let file = required_str(t, "file", "excluded_items")?;
        let written = required_str(t, "item", "excluded_items")?;
        let reason = required_str(t, "reason", "excluded_items")?;
        let selector = ItemSelector::parse(&written).with_context(|| {
            format!("[[excluded_items]] item = \"{written}\" in {file} is not an item selector")
        })?;
        out.push(ExcludedItem {
            file,
            selector,
            written,
            reason,
        });
    }
    Ok(out)
}

fn parse_provided_modules(value: Option<&toml::Value>) -> Result<Vec<ProvidedModule>> {
    let mut out = Vec::new();
    let Some(array) = value.and_then(|v| v.as_array()) else {
        return Ok(out);
    };
    for entry in array {
        let t = entry.as_table().context("[[provided]] entries are tables")?;
        let file = required_str(t, "file", "provided")?;
        let reason = required_str(t, "reason", "provided")?;
        let module = match t.get("module").and_then(|v| v.as_str()) {
            Some(m) => m.to_string(),
            // The module a file becomes, if the entry does not say: strip the
            // crate prefix and the extension, `mod`/`lib` being `index`.
            None => default_module_for(&file),
        };
        out.push(ProvidedModule {
            file,
            module,
            reason,
        });
    }
    Ok(out)
}

/// `ankql/src/parser.rs` → `parser`; `core/src/util/mod.rs` → `util/index`.
fn default_module_for(file: &str) -> String {
    let after_src = file.split_once("/src/").map(|(_, r)| r).unwrap_or(file);
    let stem = after_src.trim_end_matches(".rs");
    match stem.rsplit_once('/') {
        Some((dir, "mod")) => format!("{dir}/index"),
        _ if stem == "mod" || stem == "lib" => "index".to_string(),
        _ => stem.to_string(),
    }
}

fn required_str(t: &toml::Table, key: &str, table: &str) -> Result<String> {
    t.get(key)
        .and_then(|v| v.as_str())
        .map(String::from)
        .with_context(|| format!("[[{table}]] entries need a `{key}`"))
}

fn parse_provided_impls(value: Option<&toml::Value>) -> HashMap<String, ProvidedImpl> {
    let mut map = HashMap::new();
    if let Some(table) = value.and_then(|v| v.as_table()) {
        for (k, v) in table {
            if let Some(impl_table) = v.as_table() {
                let path = impl_table
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let reads_json = impl_table
                    .get("reads_json")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                map.insert(k.clone(), ProvidedImpl { path, reads_json });
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config::load(Path::new("transpile.toml")).unwrap()
    }

    #[test]
    fn test_load_config() {
        let config = config();
        assert_eq!(config.crates.get("ankql"), Some(&"ankql".to_string()));
        assert_eq!(config.crates.get("ankurah-proto"), Some(&"proto".to_string()));
        assert!(config.is_provided("ankurah_proto::data::EventId"));
        assert!(config.is_provided("ankurah_proto::clock::Clock"));
        assert!(config.is_provided("ankurah_proto::auth::Attested"));
        assert!(config.is_provided("ankurah_proto::transaction::TransactionId"));
        assert!(!config.is_provided("ankurah_proto::collection::CollectionId"));
        assert_eq!(
            config.provided_import_module("ankurah_proto::id::EntityId"),
            Some("./id.provided".to_string())
        );
        assert!(config.is_excluded_file("proto/src/postgres.rs"));
    }

    #[test]
    fn crate_scope_is_the_environment_table() {
        let config = config();
        let mut packages = config.packages_in_scope();
        packages.sort();
        assert_eq!(
            packages,
            vec![
                "ankql",
                "ankurah",
                "connector-local",
                "connector-websocket",
                "core",
                "proto",
                "signals",
                "storage-common",
                "storage-indexeddb",
                "storage-sqlite",
            ]
        );
        assert!(!config.is_in_scope("storage-postgres"));
        assert!(!config.is_in_scope("connector-websocket-server"));
        assert!(config.is_in_scope("connector-websocket"));
        assert_eq!(
            config.cargo_crate_for_package("connector-websocket"),
            Some("ankurah-websocket-client-wasm".to_string())
        );
    }

    #[test]
    fn signals_resolves_tokio_and_drops_reactive_graph() {
        let config = config();
        let signals = config.features_for_crate("ankurah-signals");
        // The step-7 ruling: the tokio feature is a signals default, so
        // `impl IntoBroadcastListener for UnboundedSender` exists.
        assert!(signals.is_enabled("tokio"));
        assert!(signals.is_enabled("singlethread"));
        assert!(signals.is_enabled("wasm"));
        // Cargo resolves reactive-graph ON; the recorded override takes it off.
        assert!(!signals.is_enabled("reactive-graph"));
        assert!(!signals.is_enabled("multithread"));
        let over = config
            .feature_overrides
            .iter()
            .find(|o| o.feature == "reactive-graph")
            .expect("the override is recorded");
        assert!(over.reason.contains("Leptos"));
    }

    /// Cargo's own rules for what counts as a feature, which decide whether a
    /// `#[cfg(feature = "x")]` is answered or left undecided.
    #[test]
    fn a_declared_feature_set_is_the_table_plus_default_plus_implicit_dependencies() {
        let manifest: toml::Table = r#"
            [package]
            name = "thing"
            [features]
            wasm = ["dep:wasm-bindgen"]
            [dependencies]
            wasm-bindgen = { version = "0.2", optional = true }
            tracing = { version = "0.1", optional = true }
            serde = "1.0"
        "#
        .parse()
        .unwrap();
        let mut declared = features_declared_by(&manifest);
        declared.sort();
        assert_eq!(
            declared,
            vec![
                // The table's own.
                "default".to_string(),
                // An optional dependency nothing names as `dep:` IS a feature.
                "tracing".to_string(),
                "wasm".to_string(),
            ],
            "`wasm-bindgen` is named `dep:wasm-bindgen` by `wasm`, which is how a crate says the \
             dependency is not a feature of its own; `serde` is not optional; `default` is \
             declared whether or not the table says so"
        );
    }

    /// The corpus as it stands, so a crate that starts declaring a feature the
    /// port has an opinion about moves this test.
    #[test]
    fn the_corpus_declares_what_the_config_names() {
        let config = config();
        let declared = declared_features(&config.crates, &config.paths.rust_source);
        let ankql = declared.get("ankql").expect("ankql is in [crates] and has a Cargo.toml");
        assert!(ankql.contains(&"wasm".to_string()));
        assert!(ankql.contains(&"default".to_string()));
        assert!(
            !ankql.contains(&"wasm-bindgen".to_string()),
            "ankql's only optional dependency is named `dep:wasm-bindgen` by its `wasm` feature"
        );
        // `Config::load` already refuses a `[features.<crate>]` naming anything
        // absent from these lists; this is the list it checks against.
        for (krate, named) in &config.crate_features {
            let Some(declared) = declared.get(krate) else { continue };
            for feature in named {
                assert!(declared.contains(feature), "[features.{krate}] names `{feature}`");
            }
        }
    }

    #[test]
    fn per_crate_features_differ() {
        let config = config();
        assert!(config.features_for_crate("ankurah-core").is_enabled("wasm"));
        assert!(
            !config
                .features_for_crate("ankurah-storage-common")
                .is_enabled("wasm")
        );
        assert!(
            config
                .features_for_package("connector-websocket")
                .enabled_names()
                .is_empty()
        );
    }

    #[test]
    fn provided_modules_replace_the_hardcode_list() {
        let config = config();
        let parser = config
            .provided_module("ankql/src/parser.rs")
            .expect("ankql's parser is provided");
        assert_eq!(parser.module, "parser");
        assert!(config.is_hardcoded("ankql/src/grammar.rs"));
        // The three files the step-8 rulings take OFF the list.
        assert!(!config.is_hardcoded("ankql/src/ast.rs"));
        assert!(!config.is_hardcoded("proto/src/human_id.rs"));
        assert!(!config.is_hardcoded("ankql/src/selection/sql.rs"));
        assert_eq!(
            config
                .provided_module("core/src/util/mod.rs")
                .map(|p| p.module.as_str()),
            Some("util/index")
        );
    }

    #[test]
    fn excluded_items_name_the_wasm_abi_glue() {
        let config = config();
        let in_model = config.excluded_items_in("core/src/model.rs");
        assert_eq!(in_model.len(), 5, "four js_ helpers and RefWrapper");
        let in_error = config.excluded_items_in("core/src/error.rs");
        assert_eq!(in_error.len(), 2);
        assert!(in_error.iter().all(|e| e.reason.contains("ABI")));
        assert!(config.excluded_items_in("core/src/context.rs").len() == 1);
    }

    #[test]
    fn default_module_names() {
        assert_eq!(default_module_for("ankql/src/parser.rs"), "parser");
        assert_eq!(default_module_for("core/src/util/mod.rs"), "util/index");
        assert_eq!(default_module_for("core/src/lib.rs"), "index");
        assert_eq!(
            default_module_for("storage/indexeddb-wasm/src/util/cb_future.rs"),
            "util/cb_future"
        );
    }
}

/// Every feature each in-scope crate's own `Cargo.toml` declares.
///
/// Cargo declares two kinds. The `[features]` table names them outright, and an
/// OPTIONAL dependency declares one implicitly under its own name — unless some
/// feature already refers to it as `dep:<name>`, which is Cargo's way of saying
/// "this dependency is switched on by a feature and is not one itself". Both
/// count: `ankurah-core`'s `wasm` feature names `js-sys` and `send_wrapper`
/// without `dep:`, so those ARE features of the crate and a `#[cfg]` may ask
/// about them.
///
/// An empty answer means the corpus was not where `[paths] rust_source` says,
/// which is what a unit fixture looks like; nothing is checked then.

/// Every feature ONE `Cargo.toml` declares, the way Cargo counts them.
///
/// Three sources, and a port that reads only the first gets the answer wrong:
/// the `[features]` table; `default`, which Cargo declares whether or not the
/// table names it; and every OPTIONAL dependency, which Cargo turns into an
/// implicit feature of the same name — unless some feature already names it as
/// `dep:x`, which is exactly how a crate says "this dependency is not a feature
/// of mine". `#[cfg(feature = "x")]` naming something absent from this list is
/// a question nothing answers, so getting the list wrong turns a typo into a
/// silently dropped item.
fn features_declared_by(table: &toml::Table) -> Vec<String> {
    let features = table.get("features").and_then(|f| f.as_table());
    let mut declared: Vec<String> = features.map(|f| f.keys().cloned().collect()).unwrap_or_default();
    if !declared.iter().any(|f| f == "default") {
        declared.push("default".to_string());
    }
    let mut explicit_deps: Vec<String> = Vec::new();
    if let Some(features) = features {
        for value in features.values() {
            let Some(list) = value.as_array() else { continue };
            for item in list {
                let Some(text) = item.as_str() else { continue };
                if let Some(dep) = text.strip_prefix("dep:") {
                    explicit_deps.push(dep.to_string());
                }
            }
        }
    }
    for deps in dependency_tables(table) {
        for (dep, spec) in deps {
            let optional = spec.get("optional").and_then(|o| o.as_bool()).unwrap_or(false);
            if !optional || explicit_deps.iter().any(|d| d == dep) {
                continue;
            }
            if !declared.iter().any(|f| f == dep) {
                declared.push(dep.clone());
            }
        }
    }
    declared
}

fn declared_features(
    crates: &HashMap<String, String>,
    rust_source: &Path,
) -> HashMap<String, Vec<String>> {
    let mut out = HashMap::new();
    let root = std::fs::canonicalize(rust_source).unwrap_or_else(|_| rust_source.to_path_buf());
    for entry in walkdir::WalkDir::new(&root)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "Cargo.toml")
    {
        let Ok(text) = std::fs::read_to_string(entry.path()) else { continue };
        let Ok(table) = text.parse::<toml::Table>() else { continue };
        let Some(name) = table
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
        else {
            continue;
        };
        if !crates.contains_key(name) {
            continue;
        }
        out.insert(name.to_string(), features_declared_by(&table));
    }
    out
}

/// Every table in a `Cargo.toml` that declares dependencies.
///
/// Three at the top level, and three more under EACH `[target.<selector>]` — a
/// crate that says `[target.'cfg(target_arch = "wasm32")'.dependencies]
/// getrandom = { optional = true }` has declared the implicit feature
/// `getrandom` just as surely as a top-level table would. Reading only the top
/// three made a `#[cfg(feature = "getrandom")]` a question nothing answers,
/// which is how a typo and a real feature look the same.
fn dependency_tables(table: &toml::Table) -> Vec<&toml::Table> {
    const SECTIONS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
    let mut out: Vec<&toml::Table> = Vec::new();
    for section in SECTIONS {
        if let Some(deps) = table.get(section).and_then(|d| d.as_table()) {
            out.push(deps);
        }
    }
    let Some(targets) = table.get("target").and_then(|t| t.as_table()) else { return out };
    for selector in targets.values() {
        let Some(selector) = selector.as_table() else { continue };
        for section in SECTIONS {
            if let Some(deps) = selector.get(section).and_then(|d| d.as_table()) {
                out.push(deps);
            }
        }
    }
    out
}

#[cfg(test)]
mod feature_tests {
    use super::features_declared_by;

    /// PREMISE EXTENDED 2026-09-05 (fixpass4 item 10, C6'): an optional
    /// dependency declares an implicit feature of the same name, and it does so
    /// from a TARGET-SPECIFIC table exactly as it does from a top-level one.
    /// Reading only the three top-level tables left every target-gated optional
    /// dependency out of the list, so `#[cfg(feature = "getrandom")]` was a
    /// question nothing answered — indistinguishable from a typo.
    #[test]
    fn an_optional_dependency_under_a_target_selector_is_a_feature() {
        let manifest: toml::Table = r#"
            [features]
            wasm = ["dep:js-sys"]

            [dependencies]
            serde = "1"
            tracing = { version = "0.1", optional = true }

            [target.'cfg(target_arch = "wasm32")'.dependencies]
            js-sys = { version = "0.3", optional = true }
            getrandom = { version = "0.2", optional = true }

            [target.'cfg(not(target_arch = "wasm32"))'.dev-dependencies]
            tokio = { version = "1", optional = true }
        "#
        .parse()
        .expect("the fixture is valid TOML");
        let declared = features_declared_by(&manifest);
        assert!(declared.iter().any(|f| f == "wasm"), "{:?}", declared);
        assert!(declared.iter().any(|f| f == "default"), "{:?}", declared);
        assert!(declared.iter().any(|f| f == "tracing"), "a top-level one: {:?}", declared);
        assert!(declared.iter().any(|f| f == "getrandom"), "a target-gated one: {:?}", declared);
        assert!(declared.iter().any(|f| f == "tokio"), "and a target-gated dev one: {:?}", declared);
        // `dep:js-sys` says js-sys is switched ON by a feature and is not one.
        assert!(!declared.iter().any(|f| f == "js-sys"), "{:?}", declared);
        // A dependency that is not optional declares nothing.
        assert!(!declared.iter().any(|f| f == "serde"), "{:?}", declared);
    }
}
