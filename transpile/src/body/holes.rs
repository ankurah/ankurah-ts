//! The R12 hole: what an emitted file carries where the port has no lowering
//! for a Rust shape, and the count of how many have been written.
//!
//! One spelling in one place, so a hole is greppable in emitted output and the
//! harness can hold a ledger of them — and so that "this body refused a shape"
//! is answered by the LOWERING rather than by searching the rendered text for
//! `unsupported(`, which made the emitter's own output an input.

use super::quoted;

/// Indent each line by 2 spaces
/// The text of an R12 hole: what an emitted file carries where the port has no
/// lowering for a Rust shape.
///
/// One spelling, in one place, so a hole is greppable in emitted output and the
/// harness can hold a ledger of them. `unsupported` answers `never`, so this
/// stands wherever the expression it replaces stood.
pub fn hole_text(what: &str) -> String {
    HOLES_WRITTEN.with(|n| n.set(n.get() + 1));
    format!("unsupported({})", quoted(what))
}

thread_local! {
    /// How many holes have been written since the process started.
    ///
    /// I1: "this body carries a hole" is the LOWERING's answer, and `hole_text`
    /// is the one place a hole's text is made — so counting here is counting
    /// what was lowered. Read as a delta around one body's translation. The
    /// alternative, searching the rendered text for `unsupported(`, made the
    /// emitter's own output an input: a body that mentions those characters for
    /// any other reason is not a body that refused a shape.
    static HOLES_WRITTEN: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The running count, for a caller taking a delta around a translation.
pub fn holes_written() -> usize {
    HOLES_WRITTEN.with(|n| n.get())
}
