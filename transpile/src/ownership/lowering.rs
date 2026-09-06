//! The state a body's ownership lowering carries while it is being written.
//!
//! Body translation asks two different questions of every expression: what does
//! this say in TypeScript, and what does it do to the values in scope. The
//! first is `body.rs`; the second is this directory, and it is here rather than
//! beside the syntax so that "who releases this" has one place to be answered
//! and one place to be read.
//!
//! What every one of those answers needs is a place to remember what the block
//! still owes and what the statement now being written has lifted out of
//! itself — which is what this file holds. The answers themselves are next
//! door: `locals` for what a block's `let`s and parameters bind, `matching` for
//! what a pattern's bindings own, `temporaries` for what an expression produced
//! and nothing named, `statements` for what a statement releases at its end,
//! and `places`, `closures` and `iteration` for the three shapes with rules of
//! their own.

use crate::body::BodyTranslator;
use crate::ownership;

/// What the block now being translated owes, and what it has lifted out of the
/// statement it is on.
///
/// One value per body: `BodyTranslator` holds it and every method in this
/// directory reads or writes it. It is separate from the translator's own state
/// because none of it is about what the TypeScript *says* — it is about what
/// the TypeScript has to release before it is allowed to leave a scope.
#[derive(Default)]
pub struct Lowering {
    /// The locals the statement now being translated binds and owes a release
    /// for. The block reads it to decide what its `finally` says.
    pub pending: std::cell::RefCell<Vec<ownership::Owned>>,
    /// Declarations lifted out of the statement now being translated, in order.
    /// A temporary and a `?` both produce one: a line that has to stand before
    /// the statement that needs it.
    pub prelude: std::cell::RefCell<Vec<ownership::Hoist>>,
    /// Declarations that have to stand before the statement's MOVE FLAGS, which
    /// otherwise stand before everything.
    ///
    /// J3: a flag is written first because after a `return` it would be dead
    /// code — but that puts it above everything the statement evaluates on the
    /// way to the move, and an argument that THROWS there left the flag set and
    /// the moved value released by nobody. Only what cannot itself contain the
    /// move goes here: the `prelude` can (a `?` lifts the very call that
    /// consumes), and moving the flag below that would double-drop on the error
    /// path.
    pub before_flags: std::cell::RefCell<Vec<String>>,
    /// Where the statement now being translated writes its OWN outermost
    /// expression, as a span key. A call standing there is evaluated
    /// unconditionally on the way to the flag, so its arguments can be lifted
    /// above it; one nested inside a branch, a closure or an IIFE the statement
    /// writes cannot, because `before_flags` stands above the whole statement
    /// and the argument may read a name that nested block declares.
    pub statement_tail: std::cell::Cell<Option<(usize, usize, usize, usize)>>,
    /// What the block now being translated decided about each of its locals,
    /// for the statement now being translated.
    pub stmt_dispositions:
        std::cell::RefCell<std::collections::HashMap<String, ownership::Disposition>>,
    /// The locals whose release is behind a drop flag, by the name Rust wrote,
    /// and the flag's identifier. Nested blocks read it: a move inside a branch
    /// sets the flag the enclosing block's `finally` tests.
    pub flags: std::cell::RefCell<std::collections::HashMap<String, String>>,
    /// How many temporaries and flags this body has taken, so two of them never
    /// share a name.
    pub hoisted: std::cell::Cell<usize>,
    /// The locals bound to an `OwnedClosure`. A closure that owns its captures
    /// is not a bare callable — it is invoked as `f.call(x)`, and this is how
    /// the call sites the emitter can see are written that way.
    pub owned_closure_locals: std::cell::RefCell<Vec<String>>,
    /// Of those, the ones whose body hands a capture away. Rust calls such a
    /// closure an `FnOnce` and lets it run once; the runtime's `callOnce` is
    /// what transfers the captures and marks the closure moved, so a second
    /// call is the fatal Rust would have refused at compile time.
    pub once_closure_locals: std::cell::RefCell<Vec<String>>,
    /// The by-value parameters the function's own `finally` releases. A
    /// statement that REFUSES releases what it named and did not consume, and
    /// a parameter already on this list is not one of those: releasing it a
    /// second time is the double drop, not the leak (I4).
    pub claimed_params: std::cell::RefCell<std::collections::HashSet<String>>,
    /// Every by-value parameter the function takes, claimed or not — the names
    /// a refusal may owe a release for.
    pub by_value_params: std::cell::RefCell<std::collections::HashSet<String>>,
    /// Which FORM the `select!` just written took: the value-producing one is
    /// one expression and the escaping one opens `const _bN = [`, which a
    /// `return` in front of does not parse. Recorded by the lowering, as
    /// `last_match_wrote_statements` is, because the macro's NAME cannot say —
    /// a name-based answer is wrong for every select that produces a value (H1).
    pub select_wrote_statements: std::cell::Cell<bool>,
    /// Whether the arguments being translated stand in a position whose own
    /// lowering INVOKES them. `Placement::Loose` reports a closure the emitter
    /// cannot see the call site of; these are call sites it writes itself, so
    /// the report there was false.
    pub argument_is_invoked: std::cell::Cell<bool>,
}

impl<'a> BodyTranslator<'a> {
    /// Ask the engine something the emitter needs and the source did not write.
    ///
    /// A resolution files whatever it could not settle, and the expression that
    /// was written reports that gap for itself where it is emitted. An
    /// ownership decision asks about the same expression a second time, so the
    /// record is wound back and the same gap is not counted twice.
    pub(crate) fn quietly<T>(&self, ask: impl FnOnce() -> T) -> T {
        let Some(tc) = &self.types else { return ask() };
        let mark = tc.borrow().sink.mark();
        let answer = ask();
        tc.borrow().sink.rewind(mark);
        answer
    }

    /// Take a name nothing else will use.
    pub(crate) fn fresh_hoist(&self, prefix: &str) -> String {
        let n = self.own.hoisted.get();
        self.own.hoisted.set(n + 1);
        format!("{}{}", prefix, n)
    }

    /// Translate something with a statement scope of its own.
    ///
    /// A closure body and a match arm become functions in TypeScript, and a
    /// declaration lifted out of one of them cannot stand outside it: the
    /// closure's parameter is not in scope there. So the lifted declarations
    /// come back with the text instead of escaping to the enclosing statement.
    pub fn with_own_hoists<R>(&self, f: impl FnOnce() -> R) -> (R, Vec<ownership::Hoist>) {
        let saved = std::mem::take(&mut *self.own.prelude.borrow_mut());
        let result = f();
        let lifted = std::mem::replace(&mut *self.own.prelude.borrow_mut(), saved);
        (result, lifted)
    }
}
