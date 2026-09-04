//! How much of each crate the engine cannot yet type, held to a ratchet.
//!
//! Every fallback the translator still takes files a diagnostic saying what was
//! given up (spec section 4.11), so the count per crate is a measure of the
//! engine's coverage rather than a list of failures. It only has to go down.
//! This test runs `batch` over each crate and fails when a count exceeds its
//! budget, naming the causes that grew.
//!
//! A count that legitimately moves — a step that reports a class of fallback
//! nobody was counting before, or a crate that gains code — is a decision, so
//! refreshing the budget is deliberate:
//!
//!     cd transpile && UPDATE_DIAGNOSTICS_BUDGET=1 cargo test --test diagnostics_budget

mod common;

use common::{run_batch_capturing, support_tree, transpile_dir, TempDir};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Crate label, then the source directory under the support checkout.
const CRATES: [(&str, &str); 4] = [
    ("proto", "proto/src"),
    ("ankql", "ankql/src"),
    ("signals", "signals/src"),
    ("core", "core/src"),
];

/// What one crate's run produced.
#[derive(Default)]
struct Run {
    total: usize,
    /// What the run said about the declared std surface. It is the same in
    /// every crate's run — the stubs do not change between them — and it is
    /// held to its own exact number, so a stub that starts failing to resolve
    /// is a failing test rather than a line nobody reads.
    surface: usize,
    /// Diagnostics by the reason the engine gave, so a failure says which kind
    /// of gap grew rather than only that the number did.
    by_cause: BTreeMap<String, usize>,
}

#[test]
fn diagnostics_stay_within_budget() {
    let runs: Vec<(&str, Run)> = CRATES.iter().map(|(name, dir)| (*name, measure(name, dir))).collect();
    let path = transpile_dir().join("tests/diagnostics_budget.toml");

    if std::env::var_os("UPDATE_DIAGNOSTICS_BUDGET").is_some() {
        std::fs::write(&path, render(&runs))
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
        eprintln!("updated {}", path.display());
        return;
    }

    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\nRecord it with:\n    \
             cd transpile && UPDATE_DIAGNOSTICS_BUDGET=1 cargo test --test diagnostics_budget",
            path.display()
        )
    });
    let budget: toml::Table = text.parse().expect("the budget file is not valid TOML");

    let mut moved = String::new();

    // The declared surface is one fact about `transpile/std_surface/`, so every
    // crate's run has to report the same number, and that number is held
    // exactly. The README says what those diagnostics are: the receivers and
    // the raw pointers the engine does not model. Anything else is a stub that
    // stopped resolving.
    let surface = runs[0].1.surface;
    for (name, run) in &runs {
        assert_eq!(
            run.surface, surface,
            "`{name}` saw {} surface diagnostics and `{}` saw {surface}; the stubs \
             are the same in both runs",
            run.surface, runs[0].0
        );
    }
    let recorded_surface = budget
        .get("surface")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("total"))
        .and_then(|v| v.as_integer())
        .unwrap_or_else(|| panic!("no surface budget recorded; refresh with UPDATE_DIAGNOSTICS_BUDGET=1"))
        as usize;
    if surface != recorded_surface {
        let _ = writeln!(
            moved,
            "\nstd_surface: {surface} diagnostics, recorded {recorded_surface}"
        );
    }

    for (name, run) in &runs {
        let entry = budget.get(*name).and_then(|v| v.as_table()).unwrap_or_else(|| {
            panic!("no budget recorded for `{name}`; refresh with UPDATE_DIAGNOSTICS_BUDGET=1")
        });
        let recorded = entry
            .get("total")
            .and_then(|v| v.as_integer())
            .unwrap_or_else(|| panic!("`{name}` has no `total` in the budget file"))
            as usize;
        let deltas = cause_deltas(entry, &run.by_cause);
        if run.total == recorded && deltas.is_empty() {
            continue;
        }
        let _ = writeln!(
            moved,
            "\n{name}: {} diagnostics, recorded {recorded}",
            run.total
        );
        for line in deltas {
            let _ = writeln!(moved, "    {line}");
        }
    }

    assert!(
        moved.is_empty(),
        "the diagnostics moved:{moved}\n\
         Each of these is a fallback the translator takes. The count is matched \
         exactly, in both directions: a rise is coverage lost, and a fall is \
         coverage gained that has to be recorded so the next run holds the new \
         line. An upper bound alone let a step that stopped reporting a whole \
         category pass unnoticed. Read the change, then record it with:\n    \
         cd transpile && UPDATE_DIAGNOSTICS_BUDGET=1 cargo test --test diagnostics_budget"
    );
}

/// Run one crate through `batch` and count what came back.
fn measure(crate_name: &str, src_dir: &str) -> Run {
    let out = TempDir::new(&format!("budget-{crate_name}"));
    let stderr = run_batch_capturing(&support_tree().join(src_dir), out.path(), crate_name);

    let mut run = Run::default();
    for line in stderr.lines() {
        if let Some(rest) = line.strip_prefix("DIAGNOSTICS ") {
            run.total = field(rest, "total").unwrap_or_else(|| {
                panic!("the summary line for `{crate_name}` has no total: {line}")
            });
            run.surface = field(rest, "surface").unwrap_or_else(|| {
                panic!("the summary line for `{crate_name}` has no surface count: {line}")
            });
        } else if let Some(cause) = cause_of(line) {
            *run.by_cause.entry(cause).or_default() += 1;
        }
    }
    assert_eq!(
        run.total,
        run.by_cause.values().sum::<usize>(),
        "`{crate_name}` printed {} diagnostics but listed a different number",
        run.total
    );
    run
}

/// `key=value` out of the summary line.
fn field(line: &str, key: &str) -> Option<usize> {
    line.split_whitespace()
        .find_map(|part| part.strip_prefix(key)?.strip_prefix('=')?.parse().ok())
}

/// The reason a diagnostic gives, with the names in it elided so that one gap
/// is one category however many identifiers it mentions. A listed diagnostic is
/// `  file:line:col: cause; what the translator did instead`; the run's other
/// output has no position in front of it and is skipped.
///
/// A diagnostic about the declared std surface is skipped too. It is a fact
/// about `transpile/std_surface/`, the same in every crate's run, and counting
/// it here would put a constant into each crate's coverage measure.
fn cause_of(line: &str) -> Option<String> {
    let body = line.strip_prefix("  ")?;
    let (position, message) = body.split_once(": ")?;
    if !is_position(position) || position.starts_with("std_surface/") {
        return None;
    }
    let message = message.split_once("; ").map(|(cause, _)| cause).unwrap_or(message);
    Some(elide_names(message))
}

/// Does this read as `file:line:col`?
fn is_position(text: &str) -> bool {
    let mut parts = text.rsplit(':');
    matches!(
        (parts.next(), parts.next()),
        (Some(col), Some(line)) if col.parse::<usize>().is_ok() && line.parse::<usize>().is_ok()
    )
}

/// Replace every `` `name` `` with a placeholder.
fn elide_names(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut inside = false;
    for ch in message.chars() {
        match ch {
            '`' if inside => inside = false,
            '`' => {
                inside = true;
                out.push('X');
            }
            _ if inside => {}
            _ => out.push(ch),
        }
    }
    out
}

/// How each cause moved against the recorded budget, biggest rise first.
fn cause_deltas(entry: &toml::Table, actual: &BTreeMap<String, usize>) -> Vec<String> {
    let recorded: BTreeMap<String, usize> = entry
        .get("by_cause")
        .and_then(|v| v.as_table())
        .map(|t| {
            t.iter().filter_map(|(k, v)| Some((k.clone(), v.as_integer()? as usize))).collect()
        })
        .unwrap_or_default();

    let mut rows: Vec<(i64, String)> = recorded
        .keys()
        .chain(actual.keys())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .filter_map(|cause| {
            let was = recorded.get(cause).copied().unwrap_or(0) as i64;
            let now = actual.get(cause).copied().unwrap_or(0) as i64;
            if was == now {
                return None;
            }
            Some((now - was, format!("{now:>6}  ({:+})  {cause}", now - was)))
        })
        .collect();
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    rows.into_iter().map(|(_, line)| line).collect()
}

fn render(runs: &[(&str, Run)]) -> String {
    let surface = runs[0].1.surface;
    let mut out = String::from(
        "# Diagnostics budget, written by transpile/tests/diagnostics_budget.rs.\n\
         # Each number is how many fallbacks the translator still takes in that\n\
         # crate — how much of it the engine cannot yet type. Every number is\n\
         # matched EXACTLY, in both directions: a rise is coverage lost, and a\n\
         # fall is coverage gained that has to be recorded, so a step that stops\n\
         # reporting a whole category cannot pass unnoticed. Generated: do not\n\
         # hand-edit. Refresh with:\n\
         #     cd transpile && UPDATE_DIAGNOSTICS_BUDGET=1 cargo test --test diagnostics_budget\n",
    );
    let _ = write!(
        out,
        "\n# What the declared std surface itself reports, which is the same in\n\
         # every crate's run. The README's \"Engine gaps the stubs expose\" says\n\
         # what these are; anything else is a stub that stopped resolving.\n\
         [surface]\ntotal = {surface}\n"
    );
    for (name, run) in runs {
        let _ = write!(out, "\n[{name}]\ntotal = {}\n\n[{name}.by_cause]\n", run.total);
        let mut rows: Vec<(&String, &usize)> = run.by_cause.iter().collect();
        rows.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (cause, count) in rows {
            let _ = writeln!(out, "{:?} = {count}", cause);
        }
    }
    out
}
