//! The import gate's LEDGER half: reading the recorded lists, comparing them
//! with what the tools found, and writing the file back.
//!
//! Split out of `tests/import_gate.rs`, which had grown past the 600-line rule.
//! The other half of that file lays the port out and runs the two tools over
//! it; this one turns their answers into rows a person reads.

use std::collections::BTreeSet;
use std::fmt::Write as _;

pub fn listed(recorded: &toml::Table, key: &str) -> BTreeSet<String> {
    recorded
        .get(key)
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

pub fn compare(what: &str, found: &BTreeSet<String>, listed: BTreeSet<String>, into: &mut String) {
    for row in found.difference(&listed) {
        let _ = writeln!(into, "  new — {what}: {row}");
    }
    for row in listed.difference(found) {
        let _ = writeln!(into, "  gone, take the line out: {row}");
    }
}

pub fn render(
    unresolved: &BTreeSet<String>,
    unexported: &BTreeSet<String>,
    other: &BTreeSet<String>,
    undeclared: &BTreeSet<String>,
) -> String {
    let mut out = String::from(
        "# Imports that do not resolve in the laid-out port, written by\n\
         # transpile/tests/import_gate.rs, from `bun build` AND `tsc --noEmit` over the same\n\
         # layout: bun erases a type-only import before it resolves anything, so it sees\n\
         # about half of these.\n\
         #\n\
         # R11: the layout is the port as it RUNS, which is the emission plus each package's\n\
         # hand-written half, so a row here is not necessarily an import the emitter wrote.\n\
         # The two `storage-indexeddb/database.ts:8` rows are the `[[provided]]` file's own\n\
         # import list. There is no mark on a row saying which: the reader is told here\n\
         # instead, because the file is generated and a mark inside it would be lost on the\n\
         # next refresh. A `.provided.ts` name, and any file `transpile.toml` lists under\n\
         # `[[provided]]`, is the hand-written half.\n\
         #\n\
         # `unresolved` is a specifier naming no module. Most are a `[[provided]]` module the\n\
         # port has not written yet. `unexported` is a name its module does not offer, which\n\
         # is a defect in the import list. `other` is any further complaint a tool makes\n\
         # about an IMPORT — the codes are listed in `tests/common/imports.rs` — rather than\n\
         # about a body, which has its own measure. `undeclared_dependencies` is a\n\
         # cross-package import whose package.json declares neither a dependency nor a\n\
         # peerDependency on the package it names — which resolves in this workspace and does\n\
         # not resolve when Expo installs by manifest.\n\
         #\n\
         # TypeScript reports only SYNTACTIC diagnostics for a program that has any, so a\n\
         # parse error anywhere would hide every semantic row here. `tests/parse_gate.rs`\n\
         # keeps that at zero, and a parse error is loud rather than silent.\n\
         #\n\
         # Matched EXACTLY in both directions, and a RATCHET: these lists may shrink, and a\n\
         # row that appears is a defect to fix rather than a line to record. Generated: do\n\
         # not hand-edit. Refresh with:\n\
         #     cd transpile && UPDATE_IMPORT_GATE=1 cargo test --test import_gate\n\n",
    );
    for (key, rows) in [
        ("unresolved", unresolved),
        ("unexported", unexported),
        ("other", other),
        ("undeclared_dependencies", undeclared),
    ] {
        let _ = writeln!(out, "# {}: {}", key, rows.len());
        if rows.is_empty() {
            let _ = writeln!(out, "{key} = []\n");
            continue;
        }
        let _ = writeln!(out, "{key} = [");
        for row in rows {
            let _ = writeln!(out, "  {row:?},");
        }
        out.push_str("]\n\n");
    }
    // Last, because a TOML table header owns everything below it: the counts a
    // review reads, checked against the lists above them.
    let _ = writeln!(
        out,
        "[summary]\nunresolved = {}\nunexported = {}\nother = {}\nundeclared_dependencies = {}",
        unresolved.len(),
        unexported.len(),
        other.len(),
        undeclared.len()
    );
    out
}

