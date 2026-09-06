//! Which declared types have TypeScript a PERSON wrote, and what each of those
//! files declares.
//!
//! Read once, at the start of a run, from the `[hardcode]` files and the
//! `[provided_impls]` table. Everything downstream asks the registry.

use crate::{config, registry};

/// The names this crate answers to in a written path: the TypeScript package
/// name the run was given, plus the Cargo and Rust spellings of the crate it
/// maps to, so `ankurah_proto::id::EntityId` written inside proto resolves.
/// Record which types the emitter will not write TypeScript for.
///
/// Two kinds: a `[provided_impls]` entry, whose TypeScript is a `.provided.ts`
/// file, and everything declared in a `[hardcode]` file, whose TypeScript is
/// kept as it stands. Both are still declared — their fields have types and
/// their derives register impls — but their *members* are whatever the person
/// who wrote the file wrote, so a hook must not call a method it did not emit.
pub(crate) fn mark_hand_written_types(
    registry: &mut registry::TypeRegistry,
    files: &[registry::ExtractedFile],
    config: Option<&config::Config>,
) {
    let mut ids = Vec::new();
    let mut reads_json = Vec::new();
    let mut declares_debug = Vec::new();
    // Types whose MEMBERS a person wrote, which includes every one of `ids` and
    // also a sibling crate's provided types.
    let mut members: Vec<crate::ty::TypeId> = Vec::new();
    for entry in files.iter().filter(|e| e.hand_written) {
        let Some(module) = registry.modules().lookup_file(&entry.path) else {
            continue;
        };
        let names = entry
            .file
            .structs
            .iter()
            .map(|s| s.name.clone())
            .chain(entry.file.enums.iter().map(|e| e.name.clone()));
        for name in names {
            if let Some(id) = registry.module_type(module, &name) {
                ids.push(id);
            }
        }
    }
    if let Some(cfg) = config {
        for fqn in cfg.provided_impls.keys() {
            // `ankurah_proto::id::EntityId` — the crate name, then the module
            // path the registry knows the type by. Read WITHOUT the crate name
            // this is the path inside the crate being transpiled, which is the
            // only crate whose impls this run emits. A SIBLING's provided type
            // is deliberately not marked here: "hand-written" stops an impl on
            // the type being emitted at all, and an impl THIS crate writes for a
            // sibling's type is this crate's own code — core's
            // `impl OrderedCollation for EntityId` has to be emitted, as the
            // module-level functions an impl away from its class becomes.
            let segments: Vec<String> = fqn.split("::").skip(1).map(|s| s.to_string()).collect();
            if segments.is_empty() {
                continue;
            }
            // The same type, wherever it is declared: a sibling's provided type
            // has no emitted members either, and asking only THIS crate's root
            // left 26 `${x.debug()}` calls in core against a method
            // `id.provided.ts` does not declare.
            for root in registry.sibling_crate_roots() {
                if let Ok(Some(registry::Def::Type(id))) = registry.lookup_type(root, &segments) {
                    members.push(id);
                }
            }
            if let Ok(Some(registry::Def::Type(id))) =
                registry.lookup_type(registry.crate_root(), &segments)
            {
                ids.push(id);
                // Whether the hand-written file declares a `static fromJson` is
                // something only the entry can say: the engine never reads the
                // TypeScript it did not write. Reading "hand-written" as
                // evidence of one put `Attested.fromJson` in three emitted call
                // sites where `auth.provided.ts` declares no such static.
                if cfg.provided_impls[fqn].reads_json {
                    reads_json.push(id);
                }
                if cfg.provided_impls[fqn].has_debug {
                    declares_debug.push(id);
                }
            }
        }
    }
    for id in members {
        registry.mark_members_hand_written(id);
    }
    for id in declares_debug {
        registry.mark_declares_debug(id);
    }
    for id in &ids {
        registry.mark_hand_written(*id);
        // Whatever the Rust derive said, a type whose class the port does not
        // emit has only the members the person who wrote the file wrote.
        registry.clear_reads_json(*id);
    }
    for id in reads_json {
        registry.mark_reads_json(id);
    }
}
