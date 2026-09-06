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
    /// The current-element bindings of every consuming loop this body is
    /// inside, and of those the ones the loop's own claim releases.
    ///
    /// S2: a loop over an owned sequence hands out one element per turn, and
    /// the tail release covers only what it never handed out. Where the claim
    /// saw the binding moved it releases nothing, and a statement that then
    /// REFUSES hands nothing away — so the element sat in the turn with no
    /// owner at all, reached by neither release walk. It is this frame's, so
    /// the frame's walk has to be able to see it.
    pub loop_bindings: std::cell::RefCell<std::collections::HashSet<String>>,
    pub claimed_loop_bindings: std::cell::RefCell<std::collections::HashSet<String>>,
    /// Names whose value some OTHER emitted frame already owns, so a refusal
    /// walk must leave them alone. A consuming loop aliases its sequence into
    /// `_seqN` and releases the tail from its own `finally`; releasing the name
    /// as well drops every element the loop already handed out.
    pub released_elsewhere: std::cell::RefCell<std::collections::HashSet<String>>,
    /// Which FORM the `select!` just written took: the value-producing one is
    /// one expression and the escaping one opens `const _bN = [`, which a
    /// `return` in front of does not parse. Recorded by the lowering, as
    /// `last_match_wrote_statements` is, because the macro's NAME cannot say —
    /// a name-based answer is wrong for every select that produces a value (H1).
    pub select_wrote_statements: std::cell::Cell<bool>,
    /// The root names a field was moved OUT of and the port wrote as a plain
    /// property read, in the order the reads were written.
    ///
    /// `takeField` is what stops the struct's cascade reaching a field the
    /// caller now owns, and it is not always writable: a field the runtime
    /// writes as a plain array or `Map` has no `takeField`, and neither has one
    /// whose type the engine could not settle. Where it was not written, the
    /// struct and the new owner both hold the field, so a scope that RELEASES
    /// the struct would release that field a second time. Recorded here, and
    /// read by whoever was about to claim the struct.
    pub partial_moves_written_as_reads: std::cell::RefCell<Vec<String>>,
    /// The type the next enum match's SUBJECT is really matching, where the
    /// expression's own type is not it.
    ///
    /// `Option<T>` is `T | null`, so an `Option` match whose arms test inside
    /// the payload is a null test around an enum match on the payload — and the
    /// payload's type is `T`, while the subject expression still resolves to
    /// `Option<T>`. Set for exactly one match and TAKEN when that match reads
    /// it, so nothing nested inside an arm can pick it up.
    pub payload_subject:
        std::cell::RefCell<Option<((usize, usize, usize, usize), crate::ty::Ty)>>,
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
