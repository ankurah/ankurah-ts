//! Loading the declared std and extern surface.
//!
//! Almost every type question about ankurah's code ends at a type ankurah did
//! not write: `self.inner.read().unwrap().values().cloned().collect()` touches
//! nine of them before it produces a type. Those declarations live in
//! `transpile/std_surface/` as ordinary signature-only Rust, and this module is
//! what puts them in the registry.
//!
//! It parses them with the extractor that reads ankurah's source and declares
//! them through the same `declare_file` / `resolve_file` doors, into a module
//! tree under the reserved system root. Nothing here knows the name of a std
//! type: a stub's file path says which module its items belong to, and its
//! Rust says everything else. Method bodies are `todo!()` and are never read.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::module::ModuleId;
use super::TypeRegistry;
use crate::diag::{Diag, DiagSink};
use crate::extract;
use crate::types::RustFile;

/// The directory name the surface ships under, inside the transpiler's crate.
pub const DIR_NAME: &str = "std_surface";

/// Files whose module is not the path they are written at.
///
/// std's public module layout does not match this directory's file layout:
/// `std::sync::Mutex` is declared in `std/sync/mutex.rs` and `std::iter::once`
/// in `std/iter/sources.rs`, because those directories are split for review
/// rather than for naming. `std/sync/atomic.rs` and `std/sync/mpsc.rs` are
/// real std submodules and are not listed, so they keep their own paths — and
/// `std::sync::atomic::Ordering` stays a different type from
/// `std::cmp::Ordering`. A file nobody listed is its own module, which is the
/// direction that fails loudly rather than resolving something wrongly.
const REVIEW_SPLITS: [(&str, &str); 8] = [
    ("std/sync/arc.rs", "std::sync"),
    ("std/sync/mutex.rs", "std::sync"),
    ("std/sync/rwlock.rs", "std::sync"),
    ("std/sync/once_lock.rs", "std::sync"),
    ("std/iter/traits.rs", "std::iter"),
    ("std/iter/adapters.rs", "std::iter"),
    ("std/iter/sources.rs", "std::iter"),
    ("std/thread/local_key.rs", "std::thread"),
];

/// Files holding inherent impls on types that belong to no module —
/// `impl u64 { .. }`, `impl str { .. }`, `impl<T> [T] { .. }`. The impls attach
/// to the primitive itself; the module they are read in only has to be able to
/// name the types their signatures mention, so they are read in `std`.
const PRIMITIVE_IMPLS: [&str; 2] = ["std/primitive.rs", "std/num.rs"];

/// The crate a `core::` or `alloc::` path reaches. Both re-export into `std`,
/// and ankurah writes `core::time::Duration` for `std::time::Duration`.
pub const STD_ALIASES: [&str; 2] = ["core", "alloc"];

/// One stub file: where it was read from, which module its items belong to, and
/// what the extractor made of it.
#[derive(Debug)]
pub struct StubFile {
    /// Relative to the surface directory, for diagnostics.
    pub path: String,
    /// `["std", "collections", "hash_map"]`.
    pub module: Vec<String>,
    pub file: RustFile,
}

/// The parsed surface. Parsing every stub costs the same on each crate of a run,
/// so a run parses once and declares the result into each registry it builds.
#[derive(Debug, Default)]
pub struct Surface {
    pub files: Vec<StubFile>,
    /// Files that did not parse, reported once per registry built from this
    /// surface so that a broken stub is visible in every run's diagnostics.
    unparsed: Vec<(String, String)>,
}

impl Surface {
    /// Read and parse every `.rs` file under `dir`.
    pub fn load(dir: &Path) -> Surface {
        let mut surface = Surface::default();
        let mut paths: Vec<PathBuf> = walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
            .map(|e| e.path().to_path_buf())
            .collect();
        // The walk order is the directory listing's; the registry's contents
        // must not depend on it.
        paths.sort();

        for path in paths {
            let rel = path
                .strip_prefix(dir)
                .unwrap_or(&path)
                .display()
                .to_string();
            let content = match std::fs::read_to_string(&path) {
                Ok(text) => text,
                Err(err) => {
                    surface.unparsed.push((rel, err.to_string()));
                    continue;
                }
            };
            match extract::extract_source(&rel, &content, None) {
                Ok(file) => {
                    let Some(module) = module_of(&rel) else {
                        surface.unparsed.push((
                            rel.clone(),
                            "a stub lives under `std/` or `extern/<crate>/`, and this one is \
                             under neither"
                                .to_string(),
                        ));
                        continue;
                    };
                    surface.files.push(StubFile {
                        path: rel,
                        module,
                        file,
                    });
                }
                Err(err) => surface.unparsed.push((rel, format!("{:#}", err))),
            }
        }
        surface
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// The module path a stub file's items belong to, or nothing when the file is
/// not somewhere the surface's conventions cover.
fn module_of(rel_path: &str) -> Option<Vec<String>> {
    let rel_path = rel_path.replace('\\', "/");
    if PRIMITIVE_IMPLS.contains(&rel_path.as_str()) {
        return Some(vec!["std".to_string()]);
    }
    if let Some((_, module)) = REVIEW_SPLITS.iter().find(|(file, _)| *file == rel_path) {
        return Some(module.split("::").map(|s| s.to_string()).collect());
    }
    let stem = rel_path.strip_suffix(".rs")?;
    let mut segments: Vec<String> = stem.split('/').map(|s| s.to_string()).collect();
    // `foo/mod.rs` is `foo`, exactly as it is for a crate's own files.
    if segments.last().is_some_and(|s| s == "mod") {
        segments.pop();
    }
    match segments.first().map(|s| s.as_str()) {
        // `std/vec.rs` is `std::vec`; the directory name is the crate name.
        Some("std") => Some(segments),
        // `extern/tokio/sync.rs` is `tokio::sync`: the crate is a root of its
        // own, so `tokio::sync::Mutex` and `std::sync::Mutex` stay two types.
        Some("extern") => {
            segments.remove(0);
            (!segments.is_empty()).then_some(segments)
        }
        _ => None,
    }
}

/// Declare the whole surface into `reg`, under the system root.
///
/// Two passes over the surface for the same reason the crate needs two: a
/// signature in `std/sync/mutex.rs` names `Result`, which `std/result.rs`
/// declares. The surface is taken by `&mut` because resolving a written type
/// records the answer on the file it was written in, exactly as it does for the
/// crate's own files; the recording is the same every time, so one parsed
/// surface serves every registry a run builds.
pub fn declare(reg: &mut TypeRegistry, surface: &mut Surface, sink: &DiagSink) {
    for (path, why) in &surface.unparsed {
        sink.push(Diag {
            file: stub_file_name(path),
            line: 0,
            col: 0,
            message: format!("std surface file `{}` could not be read: {}", path, why),
        });
    }

    let system = reg.system_root();
    let modules: Vec<ModuleId> = surface
        .files
        .iter()
        .map(|stub| reg.modules_mut().module_for_path(system, &stub.module))
        .collect();

    for (stub, module) in surface.files.iter().zip(&modules) {
        sink.set_file(&stub_file_name(&stub.path));
        super::build::declare_file(reg, *module, &stub.file, sink);
    }

    // `core::sync::atomic` and `alloc::vec` name what `std` names.
    let std_root = reg.modules().system_crates().get("std").copied();
    if let Some(std_root) = std_root {
        for alias in STD_ALIASES {
            reg.modules_mut().alias_child(system, alias, std_root);
        }
    }

    // Both indexes read declarations only, so they are built before the pass
    // that resolves written types — which is the pass that needs them.
    index(reg);

    let mut defaults = Vec::new();
    for (stub, module) in surface.files.iter().zip(&modules) {
        sink.set_file(&stub_file_name(&stub.path));
        super::build::resolve_param_defaults(reg, *module, &stub.file, sink, &mut defaults);
    }
    super::build::apply(reg, defaults);

    let mut updates = Vec::new();
    for (stub, module) in surface.files.iter_mut().zip(&modules) {
        sink.set_file(&stub_file_name(&stub.path));
        super::build::resolve_file(reg, *module, &mut stub.file, sink, &mut updates);
    }
    super::build::apply(reg, updates);
}

/// The name a diagnostic about a stub carries, so a failure points at the file
/// that has to change rather than at ankurah's source.
fn stub_file_name(rel: &str) -> String {
    format!("{}/{}", DIR_NAME, rel)
}

/// Index the surface twice over: every type by the full path it is declared at,
/// so emission policy can name one — `Arc` is `std::sync::Arc`, and nothing else
/// with that leaf name is — and every name by its leaf, which is how a stub that
/// writes `Formatter` with no import reaches `std::fmt::Formatter`.
fn index(reg: &mut TypeRegistry) {
    let system = reg.system_root();
    let mut found: Vec<(Vec<String>, super::Ns, String, super::Def)> = Vec::new();
    walk(reg, system, &mut Vec::new(), &mut found);
    for (module_path, ns, name, def) in found {
        reg.record_surface_name(ns, &name, def);
        if let (super::Ns::Type, super::Def::Type(id)) = (ns, def) {
            let mut path = module_path;
            path.push(name);
            reg.record_system_path(&path.join("::"), id);
        }
    }
    // A `pub use` is a path too. `std::collections::HashMap` is the name
    // ankurah writes and std's own re-export of `hash_map::HashMap`; both reach
    // the one type, and both have to be nameable.
    for (path, id) in reexports(reg, system, &mut Vec::new()) {
        reg.record_system_path(&path, id);
    }
}

fn reexports(
    reg: &TypeRegistry,
    module: ModuleId,
    prefix: &mut Vec<String>,
) -> Vec<(String, crate::ty::TypeId)> {
    let mut out = Vec::new();
    for binding in &reg.modules().get(module).uses {
        let (Some(local), super::Vis::Public) = (&binding.local, binding.vis) else {
            continue;
        };
        if let Ok(Some(super::Def::Type(id))) = reg.lookup_type(module, &binding.path) {
            let mut path = prefix.clone();
            path.push(local.clone());
            out.push((path.join("::"), id));
        }
    }
    let children: Vec<(String, ModuleId)> = reg
        .modules()
        .get(module)
        .children
        .iter()
        .map(|(n, id)| (n.clone(), *id))
        .collect();
    for (name, child) in children {
        if STD_ALIASES.contains(&name.as_str()) {
            continue;
        }
        prefix.push(name);
        out.extend(reexports(reg, child, prefix));
        prefix.pop();
    }
    out
}

fn walk(
    reg: &TypeRegistry,
    module: ModuleId,
    prefix: &mut Vec<String>,
    out: &mut Vec<(Vec<String>, super::Ns, String, super::Def)>,
) {
    for ((ns, name), item) in &reg.modules().get(module).items {
        out.push((prefix.clone(), *ns, name.clone(), item.def));
    }
    let children: Vec<(String, ModuleId)> = reg
        .modules()
        .get(module)
        .children
        .iter()
        .map(|(n, id)| (n.clone(), *id))
        .collect();
    for (name, child) in children {
        // The tree is built from file paths, so it is a tree — except for the
        // `core` and `alloc` aliases, which point back at `std`.
        if STD_ALIASES.contains(&name.as_str()) {
            continue;
        }
        prefix.push(name);
        walk(reg, child, prefix, out);
        prefix.pop();
    }
}

/// Where the surface ships. Under `cargo test` the crate directory is given;
/// an installed binary looks beside itself and then beside the crate it was
/// built from, and `--std-surface` overrides both.
pub fn default_dir(override_dir: Option<&Path>) -> PathBuf {
    if let Some(dir) = override_dir {
        return dir.to_path_buf();
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let candidate = Path::new(&manifest).join(DIR_NAME);
        if candidate.is_dir() {
            return candidate;
        }
    }
    // Beside the executable, then two directories up out of `target/<profile>/`.
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors().skip(1).take(4) {
            let candidate = ancestor.join(DIR_NAME);
            if candidate.is_dir() {
                return candidate;
            }
        }
    }
    PathBuf::from(DIR_NAME)
}

/// Run `f` with the surface at `dir`, parsed once per thread.
///
/// `syn` nodes are not `Send`, so the cache cannot be shared between threads: a
/// `batch` run has one thread and parses once for all its crates, and each test
/// thread parses once for all its tests.
pub fn with_cached<T>(dir: &Path, f: impl FnOnce(&mut Surface) -> T) -> T {
    thread_local! {
        static CACHE: std::cell::RefCell<HashMap<PathBuf, Surface>> =
            std::cell::RefCell::new(HashMap::new());
    }
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let surface = cache
            .entry(dir.to_path_buf())
            .or_insert_with(|| Surface::load(dir));
        f(surface)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_path_names_the_module_its_items_belong_to() {
        assert_eq!(module_of("std/vec.rs"), Some(vec!["std".into(), "vec".into()]));
        assert_eq!(
            module_of("std/collections/hash_map.rs"),
            Some(vec!["std".into(), "collections".into(), "hash_map".into()])
        );
        assert_eq!(
            module_of("extern/tokio/sync.rs"),
            Some(vec!["tokio".into(), "sync".into()])
        );
        assert_eq!(module_of("extern/js_sys.rs"), Some(vec!["js_sys".into()]));
    }

    #[test]
    fn the_split_files_belong_to_the_module_std_puts_them_in() {
        assert_eq!(
            module_of("std/sync/mutex.rs"),
            Some(vec!["std".into(), "sync".into()]),
            "`std::sync::Mutex`, not `std::sync::mutex::Mutex`"
        );
        assert_eq!(
            module_of("std/sync/atomic.rs"),
            Some(vec!["std".into(), "sync".into(), "atomic".into()]),
            "but `atomic` is a real submodule and keeps its own `Ordering`"
        );
        assert_eq!(
            module_of("std/iter/adapters.rs"),
            Some(vec!["std".into(), "iter".into()])
        );
    }

    #[test]
    fn a_file_outside_the_conventions_names_no_module() {
        assert_eq!(module_of("README.md"), None);
        assert_eq!(module_of("scratch/thing.rs"), None);
    }
}
