//! Where the emitted TypeScript releases what Rust would have dropped.
//!
//! Rust runs drop glue at a scope's end, at every early exit out of it, and
//! while an unwind passes through. TypeScript runs none of that, so the emitter
//! writes it: a block that owns something wraps its body in `try`/`finally` and
//! releases what it still owns in the `finally`, in reverse declaration order.
//! `return`, `?`, `break`, `continue` and a thrown fatal all leave through that
//! `finally`, which is the reason it is a `finally` and not a run of statements
//! at the end.
//!
//! Two questions decide what a block owes, and each has its own file here:
//! `glue` says what a value of some type has to release, and `moves` says which
//! of the block's locals were handed to somebody else before it ended. The rest
//! of the directory writes the answers out, one file per kind of thing that
//! owns a value: `locals` for what a block's `let`s and parameters bind,
//! `matching` for what a pattern's bindings own, `temporaries` for what an
//! expression produced and nothing named, `statements` for what a statement
//! releases at its end, and `places`, `closures` and `iteration` for the three
//! shapes with rules of their own. `lowering` holds the state they all share.

pub mod closures;
pub mod glue;
pub mod iteration;
pub mod locals;
pub mod lowering;
pub mod matching;
pub mod moves;
pub mod places;
pub mod scrutinee;
pub mod statements;
pub mod temporaries;
#[cfg(test)]
mod borrowing_tests;
#[cfg(test)]
mod callable_tests;
#[cfg(test)]
mod tests;

pub use glue::{drops_of, fresh_at_each_use, Drops};
pub use lowering::Lowering;
pub use moves::{Disposition, Dispositions, Scan};

/// One value a block holds and owes a release for.
#[derive(Debug, Clone)]
pub struct Owned {
    /// The identifier it was emitted under, which is not the name Rust wrote
    /// wherever a shadow had to be freshened.
    pub name: String,
    /// The name Rust wrote, where this is a local. A drop flag is registered
    /// under it so that a move inside a nested block can find it, and taken
    /// off again when the declaring block ends.
    pub source: Option<String>,
    pub drops: Drops,
    /// The flag a conditional move sets. Rust compiles one for exactly this
    /// case; the `finally` reads it instead of releasing unconditionally.
    pub flag: Option<String>,
    /// A temporary lifted out of an expression. It is released at the end of
    /// the statement that produced it as well as here — which only a guard
    /// tolerates, and only because its second drop is a deliberate no-op.
    pub statement_scoped: bool,
}

impl Owned {
    /// What the `finally` says about this value.
    pub fn release(&self) -> String {
        let Some(release) = self.drops.release(&self.name) else {
            return String::new();
        };
        match &self.flag {
            Some(flag) => format!("if (!{}) {}\n", flag, release),
            None => format!("{}\n", release),
        }
    }

    /// What the end of the producing statement says. Only a guard gets one:
    /// releasing a lock at the end of the statement that took it is the whole
    /// point, and a guard's second drop is free. Anything else waits for the
    /// `finally`, because dropping it twice is fatal.
    pub fn statement_release(&self) -> String {
        if !self.statement_scoped || self.drops != Drops::Guard {
            return String::new();
        }
        format!("{}.drop();\n", self.name)
    }
}

/// A declaration lifted out of the statement that needed it.
///
/// A guard produced inside an expression, and the `Result` a `?` tests, are
/// both values the statement cannot hold in place: one needs a name to be
/// released under, the other needs a test before the statement runs.
#[derive(Debug, Clone)]
pub struct Hoist {
    /// The line that stands before the statement, ending in a newline.
    pub declaration: String,
    /// What it owes a release for, where it owes one. A `?` wrapper is
    /// consumed by the `unwrap` that follows and owes nothing.
    pub owned: Option<Owned>,
}

/// `body`, with everything lifted out of it declared before it and released
/// around it.
///
/// A hoist's declaration has to stand before the text that names it, and the
/// value it declared has to be released however that text is left — which is
/// the same `try`/`finally` a block writes for its own locals, scoped to
/// whatever asked for the hoist.
pub fn hoisted(body: &str, hoists: &[Hoist]) -> String {
    let mut inner = body.to_string();
    for hoist in hoists.iter().rev() {
        let wrapped = match &hoist.owned {
            Some(owned) => wrap(&inner, owned),
            None => inner,
        };
        inner = format!("{}{}", hoist.declaration, wrapped);
    }
    inner
}

/// Wrap `body` so that `owned` is released however the block is left.
///
/// The value's declaration stays outside: a `const` declared inside the `try`
/// is not in scope in the `finally`, and hoisting it would cost the type
/// annotation and the `const`.
pub fn wrap(body: &str, owned: &Owned) -> String {
    let release = owned.release();
    if release.is_empty() {
        return body.to_string();
    }
    format!(
        "try {{\n{}}} finally {{\n{}}}\n",
        crate::body::indent(body),
        crate::body::indent(&release)
    )
}
