//! Diagnostics — the record of what the transpiler could not work out, and where.
//!
//! The engine answers a type question or refuses; it never guesses. A refusal
//! becomes a `Diag` naming the Rust file, line and column, so the person reading
//! the run can open the source and see what the engine was looking at.
//!
//! While the translator still has heuristic fallbacks (spec section 4.11), a
//! fallback firing also files a `Diag` and then behaves as it did before, so the
//! transpiled output stays comparable from one step to the next. The count is
//! the coverage metric: it is how much of a crate the engine cannot yet type.
//! The fail-loud step makes this sink fatal and deletes the fallbacks.

use std::cell::RefCell;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diag {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub message: String,
}

impl std::fmt::Display for Diag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}",
            self.file, self.line, self.col, self.message
        )
    }
}

impl Diag {
    /// Build a diagnostic at a syn span. `span-locations` is on, so the line and
    /// column are the ones in the file syn parsed.
    pub fn at(file: &str, span: proc_macro2::Span, message: impl Into<String>) -> Diag {
        let start = span.start();
        Diag {
            file: file.to_string(),
            line: start.line,
            col: start.column + 1,
            message: message.into(),
        }
    }
}

/// Where every diagnostic of one transpiler run lands.
///
/// Shared by reference and written through a `RefCell`, because the translator
/// reports from `&self` methods deep inside expression translation.
#[derive(Default)]
pub struct DiagSink {
    file: RefCell<String>,
    diags: RefCell<Vec<Diag>>,
    /// Messages already filed once, per domain: the declared surface and the
    /// crate are counted apart and must not silence each other.
    once: RefCell<HashSet<(bool, String)>>,
}

impl DiagSink {
    pub fn new() -> Self {
        Self::default()
    }

    /// Name the Rust file the following diagnostics come from.
    pub fn set_file(&self, path: &str) {
        *self.file.borrow_mut() = path.to_string();
    }

    pub fn file(&self) -> String {
        self.file.borrow().clone()
    }

    pub fn push(&self, diag: Diag) {
        self.diags.borrow_mut().push(diag);
    }

    /// Report at a span in the current file.
    pub fn report(&self, span: proc_macro2::Span, message: impl Into<String>) {
        let file = self.file.borrow().clone();
        self.push(Diag::at(&file, span, message));
    }

    /// Report at a span, but only the first time this exact message is seen.
    /// Used for a missing declaration, which is one fact however many sites hit it.
    ///
    /// The record of what has been said is kept per *domain* — the declared
    /// surface and the crate being transpiled are two — because they are two
    /// measures that must not silence each other. A stub reporting
    /// "no declaration for `Waker`" used to suppress the identical sentence
    /// about ankurah's own code, and the crate's coverage number then depended
    /// on which file the run happened to read first.
    pub fn report_once(&self, span: proc_macro2::Span, message: impl Into<String>) {
        let message = message.into();
        let file = self.file.borrow().clone();
        let key = (is_surface(&file), message.clone());
        if !self.once.borrow_mut().insert(key) {
            return;
        }
        self.report(span, message);
    }

    pub fn len(&self) -> usize {
        self.diags.borrow().len()
    }

    /// Where the record stands now, so a translation the emitter tries and then
    /// abandons can take its diagnostics back with it.
    ///
    /// The translator sometimes has to write a form out before it can tell
    /// whether the form fits — a ternary whose branch turns out to need a
    /// statement. The attempt is not a fallback anybody took, and counting it
    /// would make the coverage metric report the emitter's search rather than
    /// the engine's gaps.
    pub fn mark(&self) -> usize {
        self.len()
    }

    pub fn rewind(&self, mark: usize) {
        self.diags.borrow_mut().truncate(mark);
    }

    /// How many diagnostics are about the crate being transpiled, and how many
    /// about the declared std surface.
    ///
    /// They are two different measures and must not be added up. The crate's
    /// count is the coverage metric — how much of *this* crate the engine
    /// cannot type — and it has to stay comparable from step to step. A gap in
    /// a stub is a fact about `transpile/std_surface/`, true of every crate the
    /// transpiler will ever read, and it is reported against the stub file that
    /// has to change.
    pub fn counts(&self) -> (usize, usize) {
        let surface = self
            .diags
            .borrow()
            .iter()
            .filter(|d| is_surface(&d.file))
            .count();
        (self.len() - surface, surface)
    }

    /// Every diagnostic, sorted by file and position.
    pub fn sorted(&self) -> Vec<Diag> {
        let mut out = self.diags.borrow().clone();
        out.sort();
        out
    }

    /// Print the count and the list at the end of a run, the crate's own
    /// diagnostics apart from the declared surface's.
    pub fn print_summary(&self) {
        let diags = self.sorted();
        let (crate_count, surface_count) = self.counts();
        if diags.is_empty() {
            eprintln!("\n0 diagnostics");
            return;
        }
        eprintln!("\n{} diagnostics in this crate:", crate_count);
        for d in diags.iter().filter(|d| !is_surface(&d.file)) {
            eprintln!("  {}", d);
        }
        if surface_count > 0 {
            eprintln!(
                "\n{} diagnostics in the declared std surface (the same for every crate):",
                surface_count
            );
            for d in diags.iter().filter(|d| is_surface(&d.file)) {
                eprintln!("  {}", d);
            }
        }
    }
}

/// Does this diagnostic name a stub rather than a file of the crate being
/// transpiled?
pub fn is_surface(file: &str) -> bool {
    file.starts_with(concat!("std_surface", "/"))
}

/// Fallbacks taken where no sink is in reach.
///
/// `body::translate_expr` is a free function, called from match arms, macros
/// and control flow, none of which carry a sink. A fallback taken there is
/// still a fallback and still has a position, so it is parked here and drained
/// into the run's sink by whoever owns the file being translated. Without this
/// those fallbacks are invisible and the diagnostics count is a sample rather
/// than a measure.
pub mod pending {
    use std::cell::RefCell;

    use super::{Diag, DiagSink};

    thread_local! {
        static PARKED: RefCell<Vec<(usize, usize, String)>> = const { RefCell::new(Vec::new()) };
    }

    pub fn park(span: proc_macro2::Span, message: String) {
        let start = span.start();
        park_at(start.line, start.column + 1, message);
    }

    /// Park a fallback whose position is already known. A refusal raised where
    /// there is no type context carries no position at all, and lands at 0:0 —
    /// counted, because it is a site the engine cannot type, even though it
    /// cannot yet say where.
    pub fn park_at(line: usize, col: usize, message: String) {
        PARKED.with(|p| p.borrow_mut().push((line, col, message)));
    }

    /// Move everything parked so far into `sink`, which knows the file.
    pub fn drain(sink: &DiagSink) {
        let file = sink.file();
        PARKED.with(|p| {
            for (line, col, message) in p.borrow_mut().drain(..) {
                sink.push(Diag { file: file.clone(), line, col, message });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_span_position_one_based() {
        let ty: syn::Type = syn::parse_str("*const u8").unwrap();
        let sink = DiagSink::new();
        sink.set_file("proto/src/id.rs");
        sink.report(syn::spanned::Spanned::span(&ty), "raw pointer");
        let diags = sink.sorted();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].file, "proto/src/id.rs");
        assert_eq!(diags[0].line, 1);
        assert_eq!(diags[0].col, 1);
        assert_eq!(diags[0].message, "raw pointer");
    }

    #[test]
    fn report_once_files_one_diagnostic_per_message() {
        let ty: syn::Type = syn::parse_str("Ulid").unwrap();
        let span = syn::spanned::Spanned::span(&ty);
        let sink = DiagSink::new();
        sink.set_file("proto/src/id.rs");
        sink.report_once(span, "no declaration for type `Ulid`");
        sink.report_once(span, "no declaration for type `Ulid`");
        sink.report_once(span, "no declaration for type `Uuid`");
        assert_eq!(sink.len(), 2);
    }
}
