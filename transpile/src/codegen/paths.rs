//! Turning a Rust module path into the two things emission needs from it: the
//! fully-qualified prefix a declaration's identity is built on, and the
//! relative specifier one emitted file imports another by.

/// Convert crate path to FQN prefix
/// "proto/src/error.rs" → "ankurah_proto::error"
/// "core/src/entity.rs" → "ankurah_core::entity"
/// "ankql/src/ast.rs" → "ankql::ast"
pub(super) fn crate_path_to_fqn_prefix(crate_path: &str) -> String {
    // crate_path is like "proto/src/error.rs"
    let parts: Vec<&str> = crate_path.split('/').collect();
    if parts.len() < 3 {
        return crate_path.replace('/', "::").replace(".rs", "");
    }

    let crate_name = parts[0];
    // Map crate dir name to Rust crate name
    let rust_crate = match crate_name {
        "proto" => "ankurah_proto",
        "core" => "ankurah_core",
        "signals" => "ankurah_signals",
        "ankql" => "ankql",
        "storage-common" | "storage/common" => "ankurah_storage_common",
        "storage-sqlite" | "storage/sqlite" => "ankurah_storage_sqlite",
        "storage-postgres" | "storage/postgres" => "ankurah_storage_postgres",
        "storage-indexeddb" | "storage/indexeddb-wasm" => "ankurah_storage_indexeddb_wasm",
        other => other,
    };

    // Everything after "src/" is the module path
    let module_path = parts[2..].join("::")
        .replace(".rs", "")
        .replace("mod", "")
        .replace("lib", "");

    if module_path.is_empty() || module_path == "::" {
        rust_crate.to_string()
    } else {
        format!("{}::{}", rust_crate, module_path.trim_matches(':'))
    }
}

/// Compute relative import path from `current_module` to `target_module`.
///
/// Both are TS module specifiers like `./signal/calculated` or `./broadcast`.
/// Non-relative paths (e.g., `@ankurah/proto`) are returned unchanged.
///
/// Examples:
///   ("./signal/calculated", "./broadcast")         → "../broadcast"
///   ("./signal/calculated", "./signal/map")         → "./map"
///   ("./broadcast", "./signal/calculated")          → "./signal/calculated"
///   ("./observer/callback_observer", "./broadcast")  → "../broadcast"
pub(super) fn relative_import_path(current_module: &str, target_module: &str) -> String {
    // Only adjust paths that are relative (start with "./")
    if !target_module.starts_with("./") || !current_module.starts_with("./") {
        return target_module.to_string();
    }

    // Strip leading "./" to get bare paths like "signal/calculated" or "broadcast"
    let current = &current_module[2..];
    let target = &target_module[2..];

    // Get directory of the current module (everything before last '/')
    let current_dir = match current.rfind('/') {
        Some(pos) => &current[..pos],
        None => "", // current module is at root level
    };

    // If both are at root level, no adjustment needed
    if current_dir.is_empty() {
        return target_module.to_string();
    }

    // Get directory and filename of the target module
    let (target_dir, target_file) = match target.rfind('/') {
        Some(pos) => (&target[..pos], &target[pos + 1..]),
        None => ("", target),
    };

    // Split into path components
    let current_parts: Vec<&str> = current_dir.split('/').collect();
    let target_parts: Vec<&str> = if target_dir.is_empty() {
        Vec::new()
    } else {
        target_dir.split('/').collect()
    };

    // Find common prefix length
    let common = current_parts.iter().zip(target_parts.iter())
        .take_while(|(a, b)| a == b)
        .count();

    // Number of ".." needed to go up from current dir to common ancestor
    let ups = current_parts.len() - common;

    // Remaining target path components after common prefix
    let remaining_dirs = &target_parts[common..];

    // Build the relative path
    let mut parts: Vec<&str> = Vec::new();
    if ups == 0 {
        parts.push(".");
    } else {
        for _ in 0..ups {
            parts.push("..");
        }
    }
    for dir in remaining_dirs {
        parts.push(dir);
    }
    parts.push(target_file);

    parts.join("/")
}


/// The specifier one emitted module imports the crate's bincode codec by.
///
/// For: the codec is ONE hand-written module at the crate's root
/// (`packages/<crate>/src/codec.ts`), and every emitted module that derives
/// `Serialize`/`Deserialize` reads its `BincodeReader`/`BincodeWriter` from
/// there. Written as the literal `./codec` it asked for a codec beside itself,
/// so `core/property/backend/lww.ts` named a module five directories name and
/// none of them has: the import gate recorded five such rows as modules the
/// port owed, which writing one `codec.ts` would never have satisfied.
///
/// `current_module` is the importing module's own specifier (`./property/
/// backend/lww`); a caller that does not know it — the single-file `generate_ts`
/// used by the unit tests — gets the crate root's spelling.
pub(crate) fn codec_import_path(current_module: Option<&str>) -> String {
    relative_import_path(current_module.unwrap_or("./index"), "./codec")
}

/// The whole import line, so the module that writes it needs one line for it.
pub(crate) fn codec_import(names: &str, current_module: Option<&str>) -> String {
    format!("import {{ {} }} from '{}';\n", names, codec_import_path(current_module))
}

#[cfg(test)]
mod tests {
    use super::codec_import_path;

    /// H4: the codec is at the crate root, so a module in a subdirectory walks
    /// up to it. Written as a literal `./codec` these five directories each
    /// asked for a codec of their own.
    #[test]
    fn the_codec_is_imported_from_the_crate_root() {
        assert_eq!(codec_import_path(Some("./index")), "./codec");
        assert_eq!(codec_import_path(Some("./human_id")), "./codec");
        assert_eq!(codec_import_path(Some("./value/index")), "../codec");
        assert_eq!(codec_import_path(Some("./indexing/key_spec")), "../codec");
        assert_eq!(codec_import_path(Some("./property/backend/lww")), "../../codec");
        assert_eq!(codec_import_path(Some("./property/value/json")), "../../codec");
        // A caller with no module of its own is at the root.
        assert_eq!(codec_import_path(None), "./codec");
    }
}
