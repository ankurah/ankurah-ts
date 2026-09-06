//! The ownership reports the goldens RECORD, ratcheted down.
//!
//! For: a golden driver may record a leak the runtime reports, where the leak
//! stands on a defect that is open and named — `goldens/_driver/leaks.ts`
//! matches those exactly, in both directions, so the day one is fixed the
//! golden fails and the line comes out. That is what keeps a recorded report
//! honest about the present. It says nothing about the FUTURE: nothing stopped
//! the next pass from recording two more and calling the run green.
//!
//! So a recorded report is a DEBT. Two things make it one. `RecordedReport`
//! requires an `owes` naming the addendum item that will fix it, which
//! TypeScript enforces where the driver is written; and this ledger may only
//! SHRINK. A pass that means to add one has to say so here, in a diff Daniel
//! reads, rather than in a test that stayed green.
//!
//!     cd transpile && UPDATE_RECORDED_REPORTS=1 cargo test --test recorded_reports

mod common;

use common::{code_only, transpile_dir, TempDir};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn no_golden_records_more_ownership_reports_than_it_did() {
    let root = transpile_dir();
    let goldens = root.join("goldens");
    let mut measured: BTreeMap<String, usize> = BTreeMap::new();
    let mut entries: Vec<_> = std::fs::read_dir(&goldens)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", goldens.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    entries.sort();
    for dir in entries {
        let driver = dir.join("run.test.ts");
        if !driver.is_file() {
            continue;
        }
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .expect("a directory has a name");
        let text = std::fs::read_to_string(&driver)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", driver.display()));
        // Read as CODE: `owes` inside a comment or inside one of the report
        // strings is prose about the rule, not an entry under it.
        let count = code_only(&text).matches("owes:").count();
        if count > 0 {
            measured.insert(name, count);
        }
    }

    let ledger = root.join("tests/recorded_reports.toml");
    if std::env::var_os("UPDATE_RECORDED_REPORTS").is_some() {
        std::fs::write(&ledger, render(&measured))
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", ledger.display()));
        eprintln!("updated {}", ledger.display());
        return;
    }

    let text = std::fs::read_to_string(&ledger).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\nRecord it with:\n    \
             cd transpile && UPDATE_RECORDED_REPORTS=1 cargo test --test recorded_reports",
            ledger.display()
        )
    });
    let recorded: toml::Table = text.parse().expect("the recorded-report ledger is not valid TOML");
    let recorded: BTreeMap<String, usize> = recorded
        .get("records")
        .and_then(|v| v.as_table())
        .map(|t| {
            t.iter()
                .filter_map(|(k, v)| v.as_integer().map(|n| (k.clone(), n as usize)))
                .collect()
        })
        .unwrap_or_default();

    let mut moved = String::new();
    for (name, count) in &measured {
        match recorded.get(name) {
            Some(was) if count > was => {
                let _ = writeln!(
                    moved,
                    "\n{name} records {count} ownership reports and the ledger says {was}. A \
                     recorded report is a debt: fix the defect, or say here why the port has \
                     taken on another one."
                );
            }
            Some(was) if count < was => {
                let _ = writeln!(
                    moved,
                    "\n{name} records {count} ownership reports and the ledger says {was}. Bring \
                     the number down — that is a defect fixed."
                );
            }
            None => {
                let _ = writeln!(
                    moved,
                    "\n{name} records {count} ownership report(s) and the ledger does not name \
                     it. A golden that leaks is a debt somebody has to accept."
                );
            }
            _ => {}
        }
    }
    for (name, was) in &recorded {
        if !measured.contains_key(name) {
            let _ = writeln!(
                moved,
                "\n{name} records none now (the ledger says {was}). Take its line out."
            );
        }
    }

    assert!(
        moved.is_empty(),
        "the recorded-report ledger has moved ({} recorded across {} golden(s)):\n{moved}\n\
         Once every line above has been read and accepted, refresh with:\n    \
         cd transpile && UPDATE_RECORDED_REPORTS=1 cargo test --test recorded_reports",
        measured.values().sum::<usize>(),
        measured.len()
    );
}

fn render(measured: &BTreeMap<String, usize>) -> String {
    let mut out = String::from(
        "# How many ownership reports each golden driver RECORDS: one line per\n\
         # `expectNoOwnershipReports({ except: [..] })` entry, each of which names\n\
         # the addendum item that owes the fix. This ledger may only SHRINK.\n\
         #\n\
         # Generated: do not hand-edit. Refresh with:\n\
         #     cd transpile && UPDATE_RECORDED_REPORTS=1 cargo test --test recorded_reports\n\n\
         [records]\n",
    );
    for (name, count) in measured {
        let _ = writeln!(out, "\"{name}\" = {count}");
    }
    out
}

/// The check compares a MULTISET, so a golden that leaks TWICE where it
/// recorded ONCE fails.
///
/// For: `except` used to be matched with `includes`, which answers the same for
/// the second occurrence of a line as for the first. A defect that got worse —
/// the same value leaked on a second path, or once per turn of a loop — would
/// have gone on passing under the line that records it. This drives the check
/// itself, because nothing else can: every real golden is expected to be quiet.
#[test]
fn a_second_copy_of_a_recorded_report_is_not_covered_by_the_first() {
    // No skips: a test that quietly does nothing is what this whole file is
    // written against.
    let base = base_package().unwrap_or_else(|| {
        panic!(
            "no packages/base above {} — this test runs the leak check against the real \
             runtime and cannot prove anything without it",
            transpile_dir().display()
        )
    });
    Command::new("bun")
        .arg("--version")
        .output()
        .expect("bun is not on PATH, and this test runs the leak check under it");
    // One leak is what the driver records; two is what it does.
    let twice = run_leaking(&base, 2);
    assert!(
        twice.contains("the ownership runtime reported a problem"),
        "two leaks under a line that records one has to fail:\n{twice}"
    );
    // and the recorded one alone is still covered, so the check is not simply
    // failing on everything.
    let once = run_leaking(&base, 1);
    assert!(
        !once.contains("the ownership runtime reported a problem"),
        "the recorded leak is still covered by its line:\n{once}"
    );
}

/// Run a scratch driver that leaks `n` values of a type whose single leak the
/// driver records, and answer what bun said.
fn run_leaking(base: &Path, n: usize) -> String {
    let out = TempDir::new(&format!("recorded-reports-{n}"));
    let dir = out.path();
    write(dir.join("leaks.ts"), &read(transpile_dir().join("goldens/_driver/leaks.ts")));
    write(
        dir.join("package.json"),
        "{ \"name\": \"recorded-reports\", \"private\": true, \"type\": \"module\" }\n",
    );
    write(dir.join("bunfig.toml"), "[test]\npreload = [\"@ankurah/base/src/testing.ts\"]\n");
    let scope = dir.join("node_modules/@ankurah");
    std::fs::create_dir_all(&scope).expect("cannot create the scope directory");
    std::os::unix::fs::symlink(base, scope.join("base")).expect("cannot link @ankurah/base");
    write(
        dir.join("run.test.ts"),
        &format!(
            r#"import {{ test }} from 'bun:test';
import {{ Struct }} from '@ankurah/base';
import {{ expectNoOwnershipReports }} from './leaks.ts';

class Held extends Struct {{
  constructor(readonly n: number) {{ super(); }}
}}

test('leak {n}', () => {{
  for (let at = 0; at < {n}; at++) new Held(at);
}});

test('the recorded report', async () => {{
  await expectNoOwnershipReports({{
    except: [
      {{
        report: 'BUG: Held was garbage collected without being dropped.',
        owes: 'this test, which drives the check itself',
      }},
    ],
  }});
}});
"#
        ),
    );
    let output = Command::new("bun")
        .arg("test")
        .current_dir(dir)
        .output()
        .expect("cannot run bun");
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    text
}

fn read(path: PathBuf) -> String {
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn write(path: PathBuf, text: &str) {
    std::fs::write(&path, text).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
}

/// The runtime the scratch driver runs against, where the checkout has one.
fn base_package() -> Option<PathBuf> {
    transpile_dir()
        .ancestors()
        .map(|a| a.join("packages/base"))
        .find(|c| c.join("src/index.ts").is_file() && c.join("src/testing.ts").is_file())
}
