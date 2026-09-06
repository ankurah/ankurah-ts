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
pub mod dispositions;
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
mod guard_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod refusal_tests;
#[cfg(test)]
mod terminal_tests;

pub use glue::{drops_of, fresh_at_each_use, Drops};
pub use lowering::Lowering;
pub use dispositions::Dispositions;
pub use moves::{Disposition, Scan};

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

/// The body without the `let <flag> = false;` a dead flag left in it.
///
/// A `let`'s own claim writes the declaration into the statement stream before
/// the body is finished, so dropping the flag from the release has to take the
/// declaration with it.
fn without_declaration(body: &str, flag: &str) -> String {
    let dead = format!("let {} = false;", flag);
    let kept: Vec<&str> = body.lines().filter(|line| line.trim() != dead).collect();
    match body.ends_with('\n') {
        true if !kept.is_empty() => format!("{}\n", kept.join("\n")),
        _ => kept.join("\n"),
    }
}

/// Does this body ever set the flag — `_movedN = true` — anywhere inside it,
/// a nested closure or arm included?
///
/// The flag names the emitter writes (`_moved0`, `_moved1`, ...) appear in no
/// string literal and in no comment it emits, so the text is the whole answer.
pub fn sets_the_flag(body: &str, flag: &str) -> bool {
    body.contains(&format!("{} = true", flag))
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
    // E15: a flag says "somebody else owns this now", and a body that never
    // sets it never hands the value away — so the flag is a `let` nothing
    // assigns and a test that is always false. The disposition analysis reads
    // the SOURCE, and a move it finds may be one the lowering did not write
    // (an `if let Some(x) = value` binds a name out of the option without the
    // emitted arm setting anything). What the block really did is what the
    // block really wrote, so the flag is dropped where the body does not set
    // it and the release stands unguarded. Live at
    // `storage-indexeddb/collection.ts:686` and `core/value/cast_predicate.ts`.
    let (owned, body) = match &owned.flag {
        Some(flag) if !sets_the_flag(body, flag) => {
            (Owned { flag: None, ..owned.clone() }, without_declaration(body, flag))
        }
        _ => (owned.clone(), body.to_string()),
    };
    let (owned, body) = (&owned, body.as_str());
    let release = owned.release();
    if release.is_empty() {
        return body.to_string();
    }
    // K14: a `try` around NOTHING protects nothing. A method whose whole body
    // is `drop(x)` — `MockLiveQuery::set_last_error` in core's
    // `client_relay.rs`, and a `_` binding in signals' `broadcast.rs` — came
    // out as `try { } finally { x.drop(); }`, which is the release and four
    // lines of ceremony saying it cannot be skipped when there is nothing it
    // could be skipped by.
    if body.trim().is_empty() {
        return release;
    }
    format!(
        "try {{\n{}}} finally {{\n{}}}\n",
        crate::body::indent(body),
        crate::body::indent(&release)
    )
}

#[cfg(test)]
mod wrap_tests {
    use super::{wrap, Drops, Owned};

    fn held(flag: Option<&str>) -> Owned {
        Owned {
            name: "value".to_string(),
            source: None,
            drops: Drops::Cascade,
            flag: flag.map(str::to_string),
            statement_scoped: false,
        }
    }

    /// E15: a flag says "somebody else owns this now", and a body that never
    /// SETS it never hands the value away. The disposition analysis reads the
    /// source and may find a move the lowering did not write, which left a
    /// `let` nothing assigns beside a test that is always false — live at
    /// `storage-indexeddb/collection.ts` and `core/value/cast_predicate.ts`.
    #[test]
    fn a_flag_the_body_never_sets_is_dropped_with_its_declaration() {
        let body = "let _moved1 = false;\nread(value);\n";
        let out = wrap(body, &held(Some("_moved1")));
        assert!(!out.contains("_moved1"), "the dead flag is gone:\n{}", out);
        assert!(out.contains("dropOwned(value);"), "and the release stands:\n{}", out);
    }

    /// A flag the body DOES set keeps both.
    #[test]
    fn a_flag_the_body_sets_keeps_its_guard() {
        let body = "let _moved1 = false;\n_moved1 = true;\nhand(value);\n";
        let out = wrap(body, &held(Some("_moved1")));
        assert!(out.contains("let _moved1 = false;"), "{}", out);
        assert!(out.contains("if (!_moved1) dropOwned(value);"), "{}", out);
    }
}
