//! Configuration parsing — reads transpile.toml

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Config {
    pub paths: PathsConfig,
    pub crates: HashMap<String, String>,
    pub excluded_features: Vec<String>,
    pub excluded_files: Vec<String>,
    pub name_overrides: HashMap<String, String>,
    pub provided_impls: HashMap<String, ProvidedImpl>,
    pub hardcode_files: Vec<String>,
    /// Types from other crates that need explicit import mapping
    pub cross_crate_types: HashMap<String, String>,
    /// System types (Arc, RwLock, etc.) — foundational runtime types whose shapes
    /// are declared here so the transpiler can resolve through them.
    /// Distinct from provided_impls (subject-code types hand-ported in *.provided.ts).
    pub system_types: Vec<crate::resolve::TypeDef>,
}

#[derive(Debug)]
pub struct PathsConfig {
    pub rust_source: PathBuf,
    pub ts_target: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ProvidedImpl {
    pub module: String,
    pub path: String,
    /// If set, only these methods are provided (rest are generated)
    pub methods: Option<Vec<String>>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;

        let table: toml::Table = content.parse()
            .with_context(|| "Failed to parse transpile.toml")?;

        let paths = if let Some(paths) = table.get("paths").and_then(|v| v.as_table()) {
            PathsConfig {
                rust_source: PathBuf::from(paths.get("rust_source")
                    .and_then(|v| v.as_str()).unwrap_or("../ankurah-ts-support")),
                ts_target: PathBuf::from(paths.get("ts_target")
                    .and_then(|v| v.as_str()).unwrap_or("..")),
            }
        } else {
            PathsConfig {
                rust_source: PathBuf::from("../ankurah-ts-support"),
                ts_target: PathBuf::from(".."),
            }
        };

        let crates = parse_string_map(table.get("crates"));
        let name_overrides = parse_string_map(table.get("name_overrides"));

        let excluded_features = if let Some(ef) = table.get("excluded_features").and_then(|v| v.as_table()) {
            ef.get("skip").and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let excluded_files = if let Some(ef) = table.get("excluded_files").and_then(|v| v.as_table()) {
            ef.get("files").and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let provided_impls = parse_provided_impls(table.get("provided_impls"));

        let hardcode_files = if let Some(hc) = table.get("hardcode").and_then(|v| v.as_table()) {
            hc.get("files").and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let cross_crate_types = parse_string_map(table.get("cross_crate_types"));

        let system_types = parse_system_types(table.get("system_types"));

        Ok(Config {
            paths,
            crates,
            excluded_features,
            excluded_files,
            name_overrides,
            provided_impls,
            hardcode_files,
            cross_crate_types,
            system_types,
        })
    }

    /// Check if a fully qualified Rust type has a provided impl
    pub fn is_provided(&self, rust_fqn: &str) -> bool {
        self.provided_impls.contains_key(rust_fqn)
    }

    /// Check if a specific method on a type is provided (vs generated)
    pub fn is_method_provided(&self, rust_fqn: &str, method: &str) -> bool {
        if let Some(pi) = self.provided_impls.get(rust_fqn) {
            if let Some(methods) = &pi.methods {
                methods.iter().any(|m| m == method)
            } else {
                true // whole type is provided
            }
        } else {
            false
        }
    }

    /// Check if a file should be excluded
    pub fn is_excluded_file(&self, path: &str) -> bool {
        self.excluded_files.iter().any(|f| path.ends_with(f))
    }

    /// Check if a file is hardcoded (no generation)
    pub fn is_hardcoded(&self, path: &str) -> bool {
        self.hardcode_files.iter().any(|f| path.contains(f))
    }

    /// Get the import module path for a provided type (e.g., "./id.provided")
    pub fn provided_import_module(&self, rust_fqn: &str) -> Option<String> {
        self.provided_impls.get(rust_fqn).map(|p| {
            format!("./{}", p.path)
        })
    }

    /// Map Rust crate name to TS package
    pub fn crate_to_package(&self, crate_name: &str) -> Option<String> {
        self.crates.get(crate_name).map(|pkg| format!("@ankurah/{}", pkg))
    }
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

fn parse_provided_impls(value: Option<&toml::Value>) -> HashMap<String, ProvidedImpl> {
    let mut map = HashMap::new();
    if let Some(table) = value.and_then(|v| v.as_table()) {
        for (k, v) in table {
            if let Some(impl_table) = v.as_table() {
                let module = impl_table.get("module")
                    .and_then(|v| v.as_str())
                    .unwrap_or("provided")
                    .to_string();
                let path = impl_table.get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let methods = impl_table.get("methods")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

                map.insert(k.clone(), ProvidedImpl { module, path, methods });
            }
        }
    }
    map
}

/// Parse [system_types] section from transpile.toml into TypeDefs.
/// Each entry declares a system type's shape for the type registry:
///   [system_types.Arc]
///   deref_field = "value"
///   type_params = ["T"]
///   methods = { clone = "Arc<T>", downgrade = "Weak<T>" }
fn parse_system_types(value: Option<&toml::Value>) -> Vec<crate::resolve::TypeDef> {
    use crate::resolve::{TypeDef, TypeKind, MethodSig, parse_type_string};

    let mut types = Vec::new();
    let table = match value.and_then(|v| v.as_table()) {
        Some(t) => t,
        None => return types,
    };

    for (name, entry) in table {
        let entry = match entry.as_table() {
            Some(t) => t,
            None => continue,
        };

        let deref_field = entry.get("deref_field")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let type_params: Vec<String> = entry.get("type_params")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let mut methods = HashMap::new();
        if let Some(methods_table) = entry.get("methods").and_then(|v| v.as_table()) {
            for (method_name, ret_type_val) in methods_table {
                if let Some(ret_str) = ret_type_val.as_str() {
                    methods.insert(method_name.clone(), MethodSig {
                        params: vec![],
                        ret: parse_type_string(ret_str),
                        is_static: false,
                    });
                }
            }
        }

        types.push(TypeDef {
            name: name.clone(),
            kind: TypeKind::Struct, // system types are struct-like for resolution purposes
            fields: vec![],
            methods,
            deref_field,
            type_params,
        });
    }

    types
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        let config = Config::load(Path::new("transpile.toml")).unwrap();
        assert_eq!(config.crates.get("ankql"), Some(&"ankql".to_string()));
        assert_eq!(config.crates.get("ankurah-proto"), Some(&"proto".to_string()));
        assert!(config.is_provided("ankurah_proto::data::EventId"));
        assert!(config.is_provided("ankurah_proto::clock::Clock"));
        assert!(config.is_provided("ankurah_proto::auth::Attested"));
        assert!(config.is_provided("ankurah_proto::transaction::TransactionId"));
        assert!(!config.is_provided("ankurah_proto::collection::CollectionId"));
        assert_eq!(config.provided_import_module("ankurah_proto::id::EntityId"),
            Some("./id.provided".to_string()));
        assert_eq!(config.provided_import_module("ankurah_proto::data::EventId"),
            Some("./id.provided".to_string()));
        assert!(config.is_hardcoded("ankql/src/parser.rs"));
        assert!(config.is_hardcoded("ankql/src/ast.rs"));
        assert!(config.is_excluded_file("proto/src/postgres.rs"));

        // System types
        assert!(!config.system_types.is_empty(), "system_types should be loaded from config");
        let arc = config.system_types.iter().find(|t| t.name == "Arc");
        assert!(arc.is_some(), "Arc should be in system_types");
        let arc = arc.unwrap();
        assert_eq!(arc.deref_field, Some("value".to_string()));
        assert_eq!(arc.type_params, vec!["T".to_string()]);
        assert!(arc.methods.contains_key("clone"));
        assert!(arc.methods.contains_key("downgrade"));

        let rwlock = config.system_types.iter().find(|t| t.name == "RwLock");
        assert!(rwlock.is_some(), "RwLock should be in system_types");
        let rwlock = rwlock.unwrap();
        assert!(rwlock.methods.contains_key("write"));
        // Verify method return type was parsed correctly
        let write_ret = &rwlock.methods.get("write").unwrap().ret;
        assert!(matches!(write_ret, crate::resolve::ResolvedType::Named { name, .. } if name == "RwLockWriteGuard"));
    }
}
