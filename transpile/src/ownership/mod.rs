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

/// What an arm's pattern takes out of the value it is written for, for both
/// questions the port asks about a match.
pub mod arm_takes;
pub mod closures;
pub mod glue;
pub mod hoisting;
pub use hoisting::{hoisted, hoisted_when_refused, wrap};
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
mod if_let_tests;
#[cfg(test)]
mod lift_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod shadow_tests;
#[cfg(test)]
mod question_tests;
#[cfg(test)]
mod range_tests;
#[cfg(test)]
mod refusal_tests;
#[cfg(test)]
mod taken_tests;
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
///
/// N4: which stream that is depends on WHO claimed the value. A parameter's
/// declaration and its release are written around one body and this finds it
/// there; a `let`'s declaration goes into the statement that declared it while
/// the release wraps the REST of the block, and this — handed only the rest —
/// found nothing to take out. `storage-indexeddb/collection.ts` declared a
/// `let _moved0 = false;` whose guard had been dropped seventy lines below it.
/// The caller that holds both halves does the removal there.
pub fn without_declaration(body: &str, flag: &str) -> String {
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

/// One value a REFUSED statement owes a release for, and the flag that says
/// whether the transfer it was waiting for happened.
///
/// S1: the transfer is a fact about the emitted frame, not a mark on the value.
/// `let _movedN = false;` stands above the statement, `_movedN = true;` stands
/// immediately after whatever performs the transfer — the hoist that consumed
/// it, or the statement's own text — and the `finally` reads the flag. A `Vec`,
/// a `HashMap` and a `HashSet` are a plain array, `Map` and `Set` in the port
/// and carry no move mark of their own, so asking the value was asking
/// something that always answered "nobody has taken it".
#[derive(Debug, Clone)]
pub struct RefusalRelease {
    /// The emitted name of the value.
    pub name: String,
    /// The flag this frame declares for it.
    pub flag: String,
    /// What releasing it says, without the flag around it.
    pub release: String,
}

impl RefusalRelease {
    pub fn declaration(&self) -> String {
        format!("let {} = false;\n", self.flag)
    }

    pub fn set(&self) -> String {
        format!("{} = true;\n", self.flag)
    }

    pub fn guarded(&self) -> String {
        format!("if (!{}) {}\n", self.flag, self.release)
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
    /// The identifier this declaration introduced, where it introduced one.
    /// Read only on the path where the statement REFUSED: the `unwrap` that
    /// would have consumed a `?` wrapper never runs there, so the wrapper and
    /// everything it holds have no owner at all (I4).
    pub temp: Option<String>,
    /// Did this hoist's OWN lowering write a hole? Everything the statement
    /// lifted before it ran; its declaration is where the throw stands, so
    /// nothing after it is reached.
    pub refused: bool,
    /// Is this a value Rust had NOT yet built where the lift stands?
    ///
    /// N3: an argument lifted above a move flag is evaluated earlier than Rust
    /// evaluates it — the whole point, so that the flag can stand below every
    /// operand that can throw. Rust's own unwind drops such a temporary; here
    /// nobody owns it at all if a later operand throws before the call is
    /// reached. So it is released however the expression is left, asked of the
    /// runtime first, because the call it was lifted for may have consumed it.
    pub released_if_unreached: bool,
    /// Is this temporary a `?`'s `Result` WRAPPER?
    ///
    /// R0(3): the wrapper is released however the statement is left, but only
    /// where the text can leave before the `unwrap` that consumes it —
    /// `may_leave_before_reading` reads the text and decides. What separates
    /// this from `released_if_unreached` is what the value IS: a wrapper is
    /// always the runtime's own `Result`, so `isMoved` on it is the runtime's
    /// own honest answer, which is why S1 retires the guard for a lifted
    /// argument — an arbitrary value that may carry no mark — and not here.
    pub wrapper: bool,
    /// The move flags a LOCAL handed away inside THIS hoist owes, written
    /// immediately above this declaration.
    ///
    /// U3: a `?` evaluates the consuming call in its hoist and leaves the
    /// statement on the error path, so its flag cannot stand below the prelude
    /// — it would never be set, and the block would release what the callee
    /// took. It used to stand above the WHOLE prelude instead, which is above
    /// the arguments `lifted_above_the_flag` lifted precisely so that the flag
    /// could stand below them: `self.0.query(R::Model::collection(), args)?`
    /// marked `args` handed over and then called `R::Model::collection()`, and
    /// on that call's throw path nobody released `args`. The flag belongs to
    /// the transfer, so it travels with the hoist the transfer is written in.
    pub sets: String,
    /// Does the value this hoist built owe a release if nothing takes it?
    ///
    /// W1/X2: separate from `flag`, which is only written where something
    /// AFTER the lift can throw before the call runs. The last lift of a call
    /// needs no flag — nothing between it and the call can throw — but it
    /// still owes a release where the call is never written at all: the port
    /// refused `top_k`, so `const _b13 = orderBySpill.clone();` stands above a
    /// hole with no transfer below it and nobody to release the clone.
    pub droppable: bool,
    /// Is this temporary the PAYLOAD of a `?` on an `Option`?
    ///
    /// W14: `Option<T>` is `T | null`, so such a `?` writes no wrapper — the
    /// temporary IS the payload, and it is an arbitrary value that may be a
    /// plain array, `Map` or `Set` carrying no move mark, which is why S1
    /// forbids releasing it from the runtime guard a wrapper uses. It is
    /// released from a flag this frame declares instead, and only where the
    /// text can LEAVE before it is read: a second `?` in the same statement
    /// returns with the first one's payload still in hand, and Rust drops that
    /// temporary on the way out.
    pub payload: bool,
    /// GG5/FF8: does the statement this hoist stands above SUSPEND while the
    /// wrapper is still in hand? Read from the LOWERING when the hoist is
    /// built, because which awaits those are is a fact about the syntax and was
    /// being counted in brackets of rendered text.
    pub suspends: bool,
    /// The flag that says whether the call this temporary was lifted FOR took
    /// it, for a lift that owes a release (N3).
    ///
    /// S1: this used to be the value's own `isMoved`, which a `Vec`, `HashMap`
    /// or `HashSet` does not carry — so `new Selection(gapPredicate, _b6, _b7)`
    /// in `core/reactor/fetch_gap.ts` released the `orderBy` array the
    /// `Selection` had just taken. The flag is declared with the lift, set
    /// immediately above the statement's own text, and read in the wrap.
    pub flag: Option<String>,
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
