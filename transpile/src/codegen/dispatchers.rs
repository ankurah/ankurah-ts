//! What a file's generated dispatchers declare and name.
//!
//! For: a dispatcher is text no declaration in the parsed file carries. It names
//! the class of every impl it tests with `instanceof` and the function each
//! branch calls, and the import writer reads the file's declarations and bodies,
//! so those names reached it through nothing: `observer.ts` tested
//! `self instanceof CallbackObserver` against a name nothing had imported, and
//! called `Arc_Inner_observe` from a module that never mentioned it.

use std::collections::HashSet;

use crate::registry::TypeRegistry;
use crate::types::RustFile;

/// Add what this file's dispatchers declare to `declared`, and the names they
/// write to `referenced`.
///
/// Both scans the bodies get: the type-shaped names, and the module-level
/// functions whose camelCase the type scan skips on purpose.
pub(super) fn names_written(
    reg: &TypeRegistry,
    file: &RustFile,
    free_names: &HashSet<String>,
    declared: &mut HashSet<String>,
    referenced: &mut HashSet<String>,
) {
    let Some(module) = reg.modules().lookup_file(&file.path) else { return };
    for dispatcher in crate::emit_impls::dispatchers(reg, module, file) {
        declared.insert(dispatcher.name);
        crate::imports::collect_written_refs(&dispatcher.text, referenced);
        if !free_names.is_empty() {
            crate::imports::collect_written_named_refs(&dispatcher.text, free_names, referenced);
        }
    }
}
