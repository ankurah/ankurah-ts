//! What method resolution answered, site by site.
//!
//! For: the oracle test (spec 6.3) has rust-analyzer's answer for a sample of
//! the corpus and needs the engine's answer for the same sites to compare it
//! with. The engine's answers are only produced while a body is being
//! translated, with that body's scopes in place, so they are recorded as they
//! are used rather than recomputed afterwards.
//!
//! Recording is off unless the `resolve` subcommand turns it on, so an ordinary
//! run costs nothing.

use std::cell::RefCell;

use crate::registry::{MethodResolution, TypeRegistry};

thread_local! {
    static RECORDING: RefCell<bool> = const { RefCell::new(false) };
    static SITES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub fn start() {
    RECORDING.with(|r| *r.borrow_mut() = true);
}

/// One resolved call, as a tab-separated row:
/// `file, line, column, method, receiver, adjusted receiver, callee, result,
/// deref steps`. The steps are `from>to` pairs, comma separated.
pub fn record(
    reg: &TypeRegistry,
    file: &str,
    span: proc_macro2::Span,
    method: &str,
    found: &MethodResolution,
) {
    if !RECORDING.with(|r| *r.borrow()) {
        return;
    }
    let start = span.start();
    let receiver = match found.steps.first() {
        Some(step) => reg.describe(&step.from),
        None => reg.describe(found.receiver_type()),
    };
    let mut bound: Vec<String> = found
        .subst
        .iter()
        .map(|(name, ty)| format!("{}={}", name, reg.describe(ty)))
        .collect();
    bound.sort();
    // `|` between the two ends of a step and `;` between steps: a written Rust
    // type contains neither, where `>` and `,` are both all over one, and
    // splitting on those cut `MutexGuard<HashMap<K, V>>` into pieces.
    let steps = found
        .steps
        .iter()
        .map(|s| format!("{}|{}", reg.describe(&s.from), reg.describe(&s.to)))
        .collect::<Vec<_>>()
        .join(";");
    let row = format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        file,
        start.line,
        start.column + 1,
        method,
        receiver,
        // The receiver the callee actually sees, borrow included. Rebuilding it
        // from the auto-ref alone dropped the borrow a method declared for
        // itself — `DebugStruct::finish(&mut self)` reached from a `&mut
        // DebugStruct` takes no auto-ref and is still called on a `&mut`.
        reg.describe(&found.adjusted),
        reg.describe_callee(&found.callee),
        reg.describe(&found.ret),
        steps,
        bound.join(",")
    );
    SITES.with(|s| s.borrow_mut().push(row));
}

/// Everything recorded so far, in the order the translator asked.
pub fn rows() -> Vec<String> {
    SITES.with(|s| s.borrow().clone())
}
