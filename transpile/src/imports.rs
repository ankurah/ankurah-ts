//! Import resolution — resolves Rust `use` paths to TS import statements

use std::collections::HashSet;

/// Resolve a Rust `use` path to (TS package, [symbols])
/// e.g., "ankurah_proto::EntityId" → ("@ankurah/proto", ["EntityId"])
///       "crate::clock::Clock" → None (intra-crate, handled by type→file map)
///       "serde::Serialize" → None (skipped crate)
///       "ankurah_proto::{EntityId, EventId}" → ("@ankurah/proto", ["EntityId", "EventId"])
pub fn resolve_use_import(use_path: &str) -> Option<(String, Vec<String>)> {
    // Skip crate-internal imports
    if use_path.starts_with("crate::") || use_path.starts_with("self::") || use_path.starts_with("super::") {
        return None;
    }

    // Skip std/core/alloc
    if use_path.starts_with("std::") || use_path.starts_with("core::") || use_path.starts_with("alloc::") {
        return None;
    }

    // Skip known Rust-only crates
    let skip_crates = ["serde", "bincode", "sha2", "base64", "tokio", "futures", "log",
        "tracing", "anyhow", "thiserror", "derive_more", "itertools", "petgraph", "ulid"];
    for skip in &skip_crates {
        if use_path.starts_with(skip) {
            return None;
        }
    }

    // Extract crate name (first segment)
    let first_sep = use_path.find("::")?;
    let crate_name = &use_path[..first_sep];

    // Map to TS package
    let package = crate::name_map::map_crate_to_package(crate_name)?;

    // Extract imported symbols
    let rest = &use_path[first_sep + 2..];
    let symbols = extract_import_symbols(rest);

    Some((package.to_string(), symbols))
}

/// Extract symbol names from the rest of a use path
/// e.g., "EntityId" → ["EntityId"]
///       "clock::Clock" → ["Clock"]
///       "{EntityId, EventId}" → ["EntityId", "EventId"]
///       "*" → [] (glob import, skip for now)
fn extract_import_symbols(path: &str) -> Vec<String> {
    let path = path.trim();
    if path == "*" || path.is_empty() {
        return Vec::new();
    }

    // A group names several things at once, and only its own commas separate
    // them: `broadcast::{Broadcast, BroadcastId}, Get` inside one group names
    // `Get` here and two more inside the nested one. Splitting the flattened
    // text on every comma left the inner group's closing brace stuck to its
    // last symbol — `import { BroadcastId}, Get } from …` — which does not
    // parse, and took the rest of the file's diagnostics with it.
    if let Some(open) = path.find('{') {
        if path.ends_with('}') {
            let inner = &path[open + 1..path.len() - 1];
            return split_top_level(inner)
                .into_iter()
                .flat_map(|item| extract_import_symbols(&item))
                // A group's lowercase entries are its modules, which the port
                // does not import: `{broadcast::{..}, Get}` imports `Get`.
                .filter(|s| s.chars().next().is_some_and(|c| c.is_uppercase()))
                .collect();
        }
    }

    if let Some(last_sep) = path.rfind("::") {
        let symbol = path[last_sep + 2..].trim();
        if symbol == "*" {
            Vec::new()
        } else {
            vec![symbol.to_string()]
        }
    } else {
        vec![path.to_string()]
    }
}

/// Split on the commas that belong to this level, leaving a nested group's own
/// commas inside it.
fn split_top_level(inner: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    for c in inner.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                items.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    if !current.trim().is_empty() {
        items.push(current);
    }
    items
}

/// Extract PascalCase type names from a TS type string
pub fn collect_type_refs(ty: &str, refs: &mut HashSet<String>) {
    for word in ty.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if word.is_empty() || word.len() == 1 { continue; } // Skip single-letter generics (T, U, V, etc.)
        if word.chars().next().unwrap().is_uppercase()
            && !matches!(word, "Map" | "Set" | "Promise" | "Uint8Array" | "Array" | "Error")
        {
            refs.insert(word.to_string());
        }
    }
}

/// Every name in a body that is one of a known set.
///
/// A module-level function standing for an impl method is named in camelCase,
/// which the type scan above deliberately skips, so a call to one is found by
/// matching whole words against the functions the run emitted.
pub fn collect_named_refs(text: &str, known: &HashSet<String>, refs: &mut HashSet<String>) {
    for word in text.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if known.contains(word) {
            refs.insert(word.to_string());
        }
    }
}

pub fn is_primitive_or_base_type(ty: &str) -> bool {
    matches!(ty, "string" | "boolean" | "number" | "void" | "never" | "unknown" | "bigint"
        | "Struct" | "Enum" | "Drop" | "Arc" | "Weak" | "Mutex" | "MutexGuard"
        | "RwLock" | "RwLockReadGuard" | "RwLockWriteGuard"
        | "RefCell" | "Ref" | "RefMut"
        | "Borrow" | "BorrowMut" | "BincodeReader" | "BincodeWriter"
        | "Map" | "Set" | "Promise" | "Uint8Array" | "Array" | "Iterator" | "Result"
    )
}
