//! What every golden driver owes, read off the driver itself.
//!
//! For: a golden's last test is where it claims nothing leaked, and that claim
//! is worth exactly what the harness does when it is false. Fourteen drivers
//! wrote the claim as
//!
//! ```ts
//! test('nothing leaked and nothing was dropped twice', () => {
//!   expectNoOwnershipReports();
//! });
//! ```
//!
//! — a callback that discards the promise, so the assertion inside it rejected
//! into nothing and bun recorded a pass. Seven of the eight goldens the fourth
//! pass added were written that way, which is to say the goldens written to
//! check ownership were the ones not checking it. Two of them were leaking.
//!
//! `goldens/_driver/leaks.ts` refuses a discarded call at run time — the check
//! does not start until somebody awaits it, and `afterAll` fails the file for
//! every ticket nobody took. That catches it when the driver RUNS. This catches
//! it when anybody reads the tree, which is cheaper and names every site at
//! once.
//!
//! ```
//! cd transpile && cargo test --test golden_drivers
//! ```

mod common;

use common::transpile_dir;
use std::path::Path;

/// The call a driver ends with. Every use of it has to be awaited.
const CHECK: &str = "expectNoOwnershipReports(";

#[test]
fn every_leak_check_is_awaited() {
    let mut wrong: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for (name, text) in drivers() {
        for (number, line) in text.lines().enumerate() {
            let Some(at) = line.find(CHECK) else { continue };
            // The import names the function without calling it.
            if line.trim_start().starts_with("import ") || line.contains("} from ") {
                continue;
            }
            checked += 1;
            if !line[..at].trim_end().ends_with("await") {
                wrong.push(format!("  {name}/run.test.ts:{}: {}", number + 1, line.trim()));
            }
        }
    }

    assert!(
        checked >= 40,
        "only {checked} leak checks were found across the golden drivers, which means this test \
         is reading the wrong files rather than that the drivers stopped checking"
    );
    assert!(
        wrong.is_empty(),
        "these leak checks are not awaited, so the assertion inside them cannot fail the \
         test:\n{}\n\nWrite each as `test('…', async () => {{ await expectNoOwnershipReports(); \
         }})`.",
        wrong.join("\n")
    );
}

#[test]
fn a_driver_that_imports_the_leak_check_calls_it() {
    let mut silent: Vec<String> = Vec::new();
    for (name, text) in drivers() {
        let imports = text.lines().any(|line| line.contains("./leaks.ts"));
        let calls = text
            .lines()
            .any(|line| line.contains(CHECK) && !line.contains("} from "));
        if imports && !calls {
            silent.push(format!("  {name}/run.test.ts"));
        }
    }
    assert!(
        silent.is_empty(),
        "these drivers import the leak check and never call it:\n{}",
        silent.join("\n")
    );
}

/// Every golden's driver, as (golden name, text).
fn drivers() -> Vec<(String, String)> {
    let goldens = transpile_dir().join("goldens");
    let mut found: Vec<(String, String)> = Vec::new();
    let entries = std::fs::read_dir(&goldens)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", goldens.display()));
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let driver = dir.join("run.test.ts");
        if !driver.exists() {
            continue;
        }
        found.push((file_name(&dir), read(&driver)));
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

fn file_name(path: &Path) -> String {
    path.file_name().unwrap_or_default().to_string_lossy().into_owned()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}
