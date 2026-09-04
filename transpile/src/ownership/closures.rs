//! What a `move` closure owns, and who releases it.
//!
//! A Rust `move` closure takes its captures by value and drops them when the
//! closure itself is dropped — a listener holding an `Arc` keeps that `Arc`
//! alive for exactly as long as the listener lives. A JavaScript closure
//! captures the same values and the cascade cannot see any of them: it walks
//! own properties, and a capture is not a property. So every `move` closure
//! over a droppable value used to be a leak with nothing left that could
//! release it.
//!
//! `OwnedClosure(captures, fn)` is the runtime's answer: the captures become
//! ordinary owned fields, and dropping the closure cascades into them. It is
//! invoked as `closure.call(...)`, never as a bare call, so a call after the
//! drop is caught rather than reaching a body whose captures are gone.

use crate::ownership::Owned;

/// Where the closure's value goes, which decides who releases its captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// `(move || …)()` — created and finished in one expression, so the
    /// captures are released inside it and no runtime object is needed.
    Immediate,
    /// `let listener = move || …` — the local owns it, and the block that
    /// declared the local releases it.
    Bound,
    /// Anywhere else: an argument, a struct field, a return value. The closure
    /// is still an `OwnedClosure`, but the emitter cannot see who calls it, so
    /// the site is reported.
    Loose,
}

/// The immediately-invoked form: the body runs inside a scope that releases
/// what the closure captured, however the body is left.
///
/// This is what Rust does with a temporary closure — it is created, called, and
/// dropped in one expression, and dropping it drops the captures.
pub fn immediate(params: &str, body: &str, captures: &[Owned]) -> String {
    let mut inner = body.to_string();
    for capture in captures.iter().rev() {
        inner = crate::ownership::wrap(&inner, capture);
    }
    format!("({}) => {{\n{}}}", params, crate::body::indent(&inner))
}

/// The persistent form: the captures are listed beside the body that closes
/// over them, and from there they are the closure's own fields.
pub fn owned(captures: &[String], arrow: &str) -> String {
    format!("new OwnedClosure([{}], {})", captures.join(", "), arrow)
}

/// What to say about a closure whose call sites the emitter cannot see.
pub fn loose_report(captures: &[String]) -> String {
    format!(
        "this closure owns {} and is written as an `OwnedClosure`, so it is invoked as \
         `.call(...)` and released by whoever holds it; the emitter cannot see this call site \
         and has not rewritten it",
        captures
            .iter()
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
