//! Runs the emitted goldens against the real `@ankurah/base` runtime, and puts
//! the TypeScript compiler over every one of them.
//!
//! `expected.ts` says the emitter wrote the right text. This says the text does
//! the right thing when something executes it: every value it constructs reaches
//! a drop, and the runtime reports no ownership bug along the way. Without this,
//! "zero leaks, zero fatals" is a claim about output nobody ever ran.
//!
//! Executing is not enough on its own. bun strips TypeScript types instead of
//! checking them, so an emitted signature naming a type the runtime does not
//! export runs happily until something reaches that line, and a driver reaches
//! only the lines it calls. `tsc` reads the whole file, so it catches an
//! undefined name, a wrong argument type and an arm that falls off the end of a
//! function whether or not any driver goes near them.
//!
//! Three tests, three questions. `every_golden_has_a_driver_or_a_reason` asks
//! which goldens are executed at all, and refuses to let one drop out of the run
//! silently. `goldens_run_clean` executes the ones that have a driver.
//! `goldens_typecheck` compiles all of them.

mod common;

use common::{TempDir, run_batch, transpile_dir};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Where the base package sits under a checkout when nothing overrides it.
const BASE_UNDER_ROOT: &str = ".claude/worktrees/async-layer/packages/base";

/// Where the TypeScript compiler sits under a checkout when nothing overrides
/// it. Its `node_modules` also carries the `@types/bun` the drivers need.
const TSC_UNDER_ROOT: &str = ".claude/worktrees/async-layer/node_modules/.bin/tsc";

/// Text that means the runtime found an ownership problem, whatever bun made of
/// the exit code. `BUG:` opens every fatal, the leak registry's included;
/// `OwnershipFatal` is the class they are thrown as; and the cascade warning is
/// a `console.warn`, which on its own fails nothing and would otherwise scroll
/// past — it says the emitter handed the cascade something with no drop glue.
const OWNERSHIP_REPORTS: [&str; 3] = ["BUG:", "OwnershipFatal", "the drop cascade reached a"];

/// Which goldens are allowed to have no `run.test.ts`, and why each one has
/// none.
///
/// A driver is what makes a golden's claim about ownership checkable, so a
/// golden losing its driver has to fail rather than quietly stop being
/// executed. Everything under `goldens/` owes one unless it is named here.
///
/// Each of these pins the shape of a declaration and its derived codec. Running
/// one would prove nothing the text does not already say: the encode and decode
/// pair means something only against bytes Rust produced, and those bytes are
/// what the wire-protocol fixtures compare, not this runner.
const TEXT_ONLY: [(&str, &str); 5] = [
    (
        "struct_bincode",
        "a named-field struct and a byte newtype, with the encode/decode pair the derive writes",
    ),
    (
        "enum_payload",
        "an enum of one unit, one tuple and one named-field variant, with its variant-tagged codec",
    ),
    (
        "option_result_fields",
        "`Option<T>` fields and a method returning `Result<T, E>`; the README records its emitted \
         error construction as unvetted, so a driver would pin output nobody has read yet",
    ),
    (
        "tracing",
        "the calls the five tracing macros are written as. `@ankurah/base` exports no `tracing` \
         yet — the report for this pass carries the API it owes — so nothing can execute the \
         emitted module, and the text is what there is to pin",
    ),
    (
        "question_mark",
        "where the emitted `?` puts its early return. Nothing the golden constructs owns anything, \
         so executing it exercises no release",
    ),
];

/// The goldens whose emitted TypeScript does not compile yet, with the error
/// codes each one produces and the defect behind them.
///
/// This is a ledger of debt, matched exactly in both directions: a golden that
/// starts compiling has to come off the list, and a golden that starts
/// producing a new kind of error fails even though it was already failing. Both
/// halves matter — a fix that goes unrecorded here looks the same as no fix, and
/// a new defect hiding behind an old one is how this check would rot.
///
/// Every entry is a defect in the transpiler or in a decision the goldens' own
/// README already doubts. None of them is a reason to relax the check.
/// What each golden still fails to compile with, as one entry per error:
/// `<file>:<code>`, sorted. Every entry is a decision somebody read.
const TYPECHECK_DEBT: [(&str, &[&str], &str); 2] = [
    (
        "tracing",
        &["tracing/input.ts:TS2305"],
        "the emitted calls are right and the runtime does not export `tracing` yet. The hook \
         emits them because the alternative is the comment the port used to emit, which logged \
         nothing at all; the report for this pass carries the exact API `@ankurah/base` owes, \
         and this line goes when it lands",
    ),
    (
        "blanket_free_fn",
        &["blanket_free_fn/run.test.ts:TS2345"],
        "the driver hands `fromAny` a closure, and `fromAny` is emitted with the bound Rust \
         wrote — `L extends IntoListener` — which a closure does not implement structurally \
         in TypeScript, though the blanket impl makes it an `IntoListener` in Rust. The call \
         inside the function now goes through the run-time dispatcher and reaches every impl; \
         what is left is the signature, and what a bound with a blanket impl behind it should \
         emit as is open",
    ),
];

/// Every golden owes a driver, or a line in `TEXT_ONLY` saying why it does not.
#[test]
fn every_golden_has_a_driver_or_a_reason() {
    let goldens = transpile_dir().join("goldens");
    let all = all_goldens(&goldens);
    assert!(!all.is_empty(), "no goldens found under {}", goldens.display());

    let listed: BTreeSet<&str> = TEXT_ONLY.iter().map(|(name, _)| *name).collect();
    let mut problems = Vec::new();

    for name in &all {
        let driven = goldens.join(name).join("run.test.ts").is_file();
        match (driven, listed.contains(name.as_str())) {
            (false, false) => problems.push(format!(
                "{name} has no run.test.ts. Write one, or add {name} to TEXT_ONLY in this file \
                 with the reason nothing can execute it."
            )),
            (true, true) => problems.push(format!(
                "{name} now has a run.test.ts but is still listed in TEXT_ONLY as impossible to \
                 execute. Take it off the list."
            )),
            _ => {}
        }
    }
    for (name, _) in TEXT_ONLY {
        if !all.iter().any(|g| g == name) {
            problems.push(format!(
                "TEXT_ONLY names {name}, and there is no goldens/{name}. Take it off the list."
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "the set of goldens this runner executes has moved:\n\n{}\n",
        problems.join("\n")
    );
}

#[test]
fn goldens_run_clean() {
    require_bun();
    let base = base_package();
    let goldens = transpile_dir().join("goldens");
    let names: Vec<String> = all_goldens(&goldens)
        .into_iter()
        .filter(|name| goldens.join(name).join("run.test.ts").is_file())
        .collect();
    assert!(
        !names.is_empty(),
        "no golden under {} has a run.test.ts, so this test checks nothing. \
         Either every driver was deleted, or the goldens moved.",
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

/// Compile every golden — driven or not — and hold the result against the
/// ledger of what does not compile yet.
#[test]
fn goldens_typecheck() {
    let base = base_package();
    let tsc = typescript_compiler();
    let goldens = transpile_dir().join("goldens");
    let names = all_goldens(&goldens);
    assert!(!names.is_empty(), "no goldens found under {}", goldens.display());

    let root = TempDir::new("golden-typecheck");
    for name in &names {
        let dir = root.path().join(name);
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|e| panic!("cannot create {} for golden {name}: {e}", dir.display()));
        run_batch(&goldens.join(name), &dir, name);
        let emitted = dir.join("input.ts");
        assert!(
            emitted.is_file(),
            "batch wrote no input.ts for golden {name}, so there is nothing to compile"
        );
        if goldens.join(name).join("run.test.ts").is_file() {
            copy(&goldens.join(name).join("run.test.ts"), &dir.join("run.test.ts"), name);
            copy(&transpile_dir().join("goldens/_driver/leaks.ts"), &dir.join("leaks.ts"), name);
        }
        // A derived codec imports `./codec`, which is a hand-written module in
        // each emitted package rather than something batch writes. Compiling
        // against the real one is what makes `writeU64(this.id)` a checked call.
        copy(&codec_module(&base), &dir.join("codec.ts"), name);
    }
    link_base(root.path(), &base, "typecheck");
    write_tsconfig(root.path(), &tsc);

    let output = Command::new(&tsc)
        .arg("--noEmit")
        .arg("--project")
        .arg(".")
        .current_dir(root.path())
        .output()
        .unwrap_or_else(|e| panic!("cannot run {}: {e}", tsc.display()));
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));

    let mut problems = Vec::new();
    for name in &names {
        let codes = error_codes(&text, name);
        let listed = TYPECHECK_DEBT.iter().find(|(golden, ..)| golden == name);
        match (codes.is_empty(), listed) {
            (true, None) => {}
            (false, None) => problems.push(format!(
                "{name} no longer compiles. Fix the emitter, or — if the output is right and the \
                 check is wrong — say so in TYPECHECK_DEBT. tsc said:\n{}",
                errors_for(&text, name)
            )),
            (true, Some((_, _, why))) => problems.push(format!(
                "{name} compiles now, and TYPECHECK_DEBT still says it does not: {why}\n\
                 Take it off the list."
            )),
            (false, Some((_, expected, why))) => {
                let mut expected: Vec<String> = expected.iter().map(|c| c.to_string()).collect();
                expected.sort();
                if codes != expected {
                    problems.push(format!(
                        "{name} fails to compile with {codes:?}, and TYPECHECK_DEBT records \
                         {expected:?}: {why}\ntsc said:\n{}",
                        errors_for(&text, name)
                    ));
                }
            }
        }
    }
    for (name, _, _) in TYPECHECK_DEBT {
        if !names.iter().any(|g| g == name) {
            problems.push(format!(
                "TYPECHECK_DEBT names {name}, and there is no goldens/{name}. Take it off the list."
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "{} of {} golden(s) compile differently than recorded:\n\n{}\n\nthe whole tsc run said:\n{text}",
        problems.len(),
        names.len(),
        problems.join("\n\n")
    );
}

/// Every golden: a directory holding Rust to transpile and the TypeScript
/// somebody decided it must produce.
///
/// A directory with an `input.rs` and no `expected.ts` is a golden being
/// written, not a golden. Nobody has read its output yet, so there is nothing
/// for a driver to assert about and nothing to hold to a compiler;
/// `idiom_goldens` is the test that fails it, and it prints the output to save.
fn all_goldens(goldens: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(goldens)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", goldens.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|e| panic!("cannot read an entry under {}: {e}", goldens.display()))
                .path()
        })
        .filter(|p| p.join("input.rs").is_file() && p.join("expected.ts").is_file())
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
    link_base(dir, base, name);
}

/// Put `@ankurah/base` where an import of it resolves from `dir`.
fn link_base(dir: &Path, base: &Path, name: &str) {
    let scope = dir.join("node_modules/@ankurah");
    std::fs::create_dir_all(&scope)
        .unwrap_or_else(|e| panic!("cannot create {} for golden {name}: {e}", scope.display()));
    std::os::unix::fs::symlink(base, scope.join("base"))
        .unwrap_or_else(|e| panic!("cannot link {} into {} for golden {name}: {e}", base.display(), scope.display()));
}

/// The compiler settings the goldens are held to: the port's own, from the
/// checkout's `tsconfig.json`, with the paths pointed at the scratch tree.
/// `@types/bun` sits beside the compiler, which is what types the drivers'
/// `bun:test` imports and the leak check's `Bun.gc`.
fn write_tsconfig(dir: &Path, tsc: &Path) {
    let types = tsc
        .parent()
        .and_then(|bin| bin.parent())
        .unwrap_or_else(|| panic!("{} is not inside a node_modules/.bin", tsc.display()))
        .join("@types");
    let text = format!(
        r#"{{
  "compilerOptions": {{
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "skipLibCheck": true,
    "allowImportingTsExtensions": true,
    "noEmit": true,
    "lib": ["ES2021"],
    "types": ["bun"],
    "typeRoots": [{types:?}]
  }},
  "include": ["**/*.ts"]
}}
"#
    );
    write(&dir.join("tsconfig.json"), &text, "typecheck");
}

/// The TypeScript error codes tsc reported against one golden's files.
/// Every error a golden produced, as `file:code` — one entry per error, not a
/// set of the codes seen.
///
/// A set of codes hid an error: a second `TS2345` somewhere else in the same
/// golden left the set unchanged, so the ledger went on saying the golden
/// failed in exactly the way it was recorded as failing. The position's line
/// and column are dropped, because a line moving is not a new error and the
/// ledger would otherwise have to be rewritten for every emitter change.
fn error_codes(output: &str, name: &str) -> Vec<String> {
    let mut out: Vec<String> = error_lines(output, name)
        .filter_map(|line| {
            let (position, rest) = line.split_once(": error ")?;
            let file = position.split('(').next()?;
            Some(format!("{}:{}", file, rest.split(':').next()?.trim()))
        })
        .collect();
    out.sort();
    out
}

fn errors_for(output: &str, name: &str) -> String {
    let lines: Vec<&str> = error_lines(output, name).collect();
    lines.join("\n")
}

/// tsc writes one `path(line,col): error TSxxxx: message` per problem, with
/// continuation lines indented under it. Paths are relative to where it ran,
/// which is the scratch tree, so a golden owns the lines under its own name.
fn error_lines<'a>(output: &'a str, name: &'a str) -> impl Iterator<Item = &'a str> {
    let prefix = format!("{name}/");
    output
        .lines()
        .filter(move |line| line.starts_with(&prefix) && line.contains(": error TS"))
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

/// The bincode reader and writer a derived codec imports as `./codec`. It is
/// written by hand once per emitted package rather than by the transpiler, and
/// `proto`'s copy is the one the wire protocol is defined against.
fn codec_module(base: &Path) -> PathBuf {
    let codec = base
        .parent()
        .unwrap_or_else(|| panic!("{} has no parent to find proto beside", base.display()))
        .join("proto/src/codec.ts");
    assert!(
        codec.is_file(),
        "cannot find {}. A golden with a derived codec imports `./codec`, so compiling one \
         needs the real reader and writer.",
        codec.display()
    );
    codec
}

/// The TypeScript compiler the goldens are checked with. `ANKURAH_TSC_PATH`
/// overrides; otherwise look beside one of our own ancestors, the same way the
/// base package is found. A missing compiler is a failure and never a skip, for
/// the same reason: a check that quietly did not run is worse than no check.
fn typescript_compiler() -> PathBuf {
    if let Some(given) = std::env::var_os("ANKURAH_TSC_PATH") {
        let given = PathBuf::from(given);
        assert!(given.is_file(), "ANKURAH_TSC_PATH does not name a file: {}", given.display());
        return given;
    }
    for ancestor in transpile_dir().ancestors() {
        let candidate = ancestor.join(TSC_UNDER_ROOT);
        if candidate.is_file() {
            return candidate;
        }
        let installed = ancestor.join("node_modules/.bin/tsc");
        if installed.is_file() {
            return installed;
        }
    }
    panic!(
        "cannot find {TSC_UNDER_ROOT} or node_modules/.bin/tsc above {}. This test compiles the \
         emitted goldens, so it needs a TypeScript compiler; run `bun install` in the checkout, \
         or set ANKURAH_TSC_PATH.",
        transpile_dir().display()
    );
}

fn require_bun() {
    match Command::new("bun").arg("--version").output() {
        Ok(probe) if probe.status.success() => {}
        Ok(probe) => panic!("`bun --version` failed ({}):\n{}", probe.status, String::from_utf8_lossy(&probe.stderr)),
        Err(e) => panic!("cannot run `bun`: {e}. This test executes the emitted TypeScript, so bun has to be on PATH."),
    }
}
