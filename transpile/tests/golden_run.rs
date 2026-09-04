//! Runs the emitted goldens against the real `@ankurah/base` runtime.
//!
//! `expected.ts` says the emitter wrote the right text. This says the text does
//! the right thing when something executes it: every value it constructs reaches
//! a drop, and the runtime reports no ownership bug along the way. Without this,
//! "zero leaks, zero fatals" is a claim about output nobody ever ran.
//!
//! A golden opts in by carrying a `run.test.ts` beside its `input.rs`. One whose
//! point is the shape of the text alone — `struct_bincode` and its kind — never
//! gets one and is skipped here.

mod common;

use common::{TempDir, run_batch, transpile_dir};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Where the base package sits under a checkout when nothing overrides it.
const BASE_UNDER_ROOT: &str = ".claude/worktrees/async-layer/packages/base";

/// Text that means the runtime found an ownership problem, whatever bun made of
/// the exit code. `BUG:` opens every fatal, the leak registry's included;
/// `OwnershipFatal` is the class they are thrown as; and the cascade warning is
/// a `console.warn`, which on its own fails nothing and would otherwise scroll
/// past — it says the emitter handed the cascade something with no drop glue.
const OWNERSHIP_REPORTS: [&str; 3] = ["BUG:", "OwnershipFatal", "the drop cascade reached a"];

#[test]
fn goldens_run_clean() {
    require_bun();
    let base = base_package();
    let goldens = transpile_dir().join("goldens");
    let names = drivable(&goldens);
    assert!(
        !names.is_empty(),
        "no golden under {} has a run.test.ts, so this test checks nothing. \
         Either every driver was deleted, or drivable() stopped recognising them.",
        goldens.display()
    );

    let mut failures = Vec::new();
    for name in &names {
        if let Some(report) = run_one(&goldens, name, &base) {
            failures.push(report);
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} driven golden(s) did not run clean:\n\n{}",
        failures.len(),
        names.len(),
        failures.join("\n")
    );
}

/// Every golden that has both a source to transpile and a driver to run it.
fn drivable(goldens: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(goldens)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", goldens.display()))
        .map(|entry| entry.unwrap_or_else(|e| panic!("cannot read an entry under {}: {e}", goldens.display())).path())
        .filter(|p| p.join("input.rs").is_file() && p.join("run.test.ts").is_file())
        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Transpile one golden into a scratch package, run its driver, and hand back a
/// report when either the run failed or the runtime said something.
fn run_one(goldens: &Path, name: &str, base: &Path) -> Option<String> {
    let golden = goldens.join(name);
    let out = TempDir::new(&format!("golden-run-{name}"));
    run_batch(&golden, out.path(), name);

    let emitted = out.path().join("input.ts");
    if !emitted.is_file() {
        return Some(format!("── {name} ──\nbatch wrote no input.ts, so the driver has nothing to import.\n"));
    }
    scaffold(out.path(), base, &golden, name);

    let output = Command::new("bun")
        .arg("test")
        .current_dir(out.path())
        .output()
        .unwrap_or_else(|e| panic!("cannot run bun for golden {name}: {e}"));
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));

    let reported: Vec<&str> = OWNERSHIP_REPORTS.iter().copied().filter(|m| text.contains(m)).collect();
    if output.status.success() && reported.is_empty() {
        return None;
    }
    let ts = std::fs::read_to_string(&emitted).unwrap_or_else(|e| format!("(cannot read {}: {e})", emitted.display()));
    Some(format!(
        "── {name} ──\nbun exited {}; ownership reports in the output: {reported:?}\n\n\
         bun test said:\n{text}\n\nthe emitted input.ts was:\n{ts}\n",
        output.status
    ))
}

/// The scratch package around one emitted golden: the driver, the shared leak
/// check, and the wiring that makes `@ankurah/base` and its test hooks resolve.
fn scaffold(dir: &Path, base: &Path, golden: &Path, name: &str) {
    copy(&golden.join("run.test.ts"), &dir.join("run.test.ts"), name);
    copy(&transpile_dir().join("goldens/_driver/leaks.ts"), &dir.join("leaks.ts"), name);
    write(&dir.join("package.json"), "{ \"name\": \"golden-run\", \"private\": true, \"type\": \"module\" }\n", name);
    write(&dir.join("bunfig.toml"), "[test]\npreload = [\"@ankurah/base/src/testing.ts\"]\n", name);

    let scope = dir.join("node_modules/@ankurah");
    std::fs::create_dir_all(&scope)
        .unwrap_or_else(|e| panic!("cannot create {} for golden {name}: {e}", scope.display()));
    std::os::unix::fs::symlink(base, scope.join("base"))
        .unwrap_or_else(|e| panic!("cannot link {} into {} for golden {name}: {e}", base.display(), scope.display()));
}

fn copy(from: &Path, to: &Path, name: &str) {
    std::fs::copy(from, to)
        .unwrap_or_else(|e| panic!("cannot copy {} to {} for golden {name}: {e}", from.display(), to.display()));
}

fn write(path: &Path, contents: &str, name: &str) {
    std::fs::write(path, contents).unwrap_or_else(|e| panic!("cannot write {} for golden {name}: {e}", path.display()));
}

/// The runtime the emitted goldens import. `ANKURAH_BASE_PATH` overrides;
/// otherwise look for the async-layer worktree beside one of our own ancestors,
/// which finds it from the main checkout and from a git worktree alike. A
/// missing package is a failure and never a skip: a golden run that quietly did
/// not happen is the thing this test exists to prevent.
fn base_package() -> PathBuf {
    if let Some(given) = std::env::var_os("ANKURAH_BASE_PATH") {
        let given = PathBuf::from(given);
        assert!(
            is_base_package(&given),
            "ANKURAH_BASE_PATH does not name a base package — {} has no src/index.ts and src/testing.ts",
            given.display()
        );
        return given;
    }
    for ancestor in transpile_dir().ancestors() {
        let candidate = ancestor.join(BASE_UNDER_ROOT);
        if is_base_package(&candidate) {
            return candidate;
        }
    }
    panic!(
        "cannot find {BASE_UNDER_ROOT} above {}. The golden run executes the emitted \
         TypeScript against the real runtime, so it needs that package's src/index.ts \
         and src/testing.ts; set ANKURAH_BASE_PATH to point at it.",
        transpile_dir().display()
    );
}

/// Both files the scratch package depends on: what a golden imports, and what
/// bunfig preloads to install the ownership test hooks.
fn is_base_package(dir: &Path) -> bool {
    dir.join("src/index.ts").is_file() && dir.join("src/testing.ts").is_file()
}

fn require_bun() {
    match Command::new("bun").arg("--version").output() {
        Ok(probe) if probe.status.success() => {}
        Ok(probe) => panic!("`bun --version` failed ({}):\n{}", probe.status, String::from_utf8_lossy(&probe.stderr)),
        Err(e) => panic!("cannot run `bun`: {e}. This test executes the emitted TypeScript, so bun has to be on PATH."),
    }
}
