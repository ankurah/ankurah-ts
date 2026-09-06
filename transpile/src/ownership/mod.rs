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
mod question_tests;
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

/// `body`, with everything lifted out of it declared before it and released
/// around it.
///
/// A hoist's declaration has to stand before the text that names it, and the
/// value it declared has to be released however that text is left — which is
/// the same `try`/`finally` a block writes for its own locals, scoped to
/// whatever asked for the hoist.
/// The same, for a statement one of whose HOISTS refused.
///
/// The `unwrap` that consumes a `?` wrapper stands in the statement's own text,
/// and on this path that text is never reached: the hole in a later hoist's
/// declaration throws first. So every temporary the statement lifted is left
/// with no owner, along with whatever it holds — two Tokens and a Result in the
/// shape the review found. Each is released however the statement is left, and
/// each release ASKS the runtime whether the value still has an owner, because
/// a hoist standing before the refusal may have been consumed by another that
/// also ran (`f(g()?)?`): `isMoved` and `isDropped` are the runtime's own answer
/// and are the only honest test the emitter has.
///
/// `after` is what the statement owes for the source values it named and did
/// not consume; it goes in the outermost `finally`, below the temporaries,
/// which is the order Rust unwinds in.
pub fn hoisted_when_refused(body: &str, hoists: &[Hoist], after: &str) -> String {
    let mut inner = body.to_string();
    for hoist in hoists.iter().rev() {
        let wrapped = match (&hoist.owned, &hoist.temp) {
            (Some(owned), _) => wrap(&inner, owned),
            // The hoist that REFUSED declared nothing that ran: its own
            // declaration is where the throw stands.
            (None, Some(temp)) if hoist.flag.is_some() && !hoist.refused => {
                wrap_flagged(&inner, temp, hoist)
            }
            // S1: unguarded. The statement's own text never runs on this path,
            // so the only thing that can have taken this temporary is another
            // hoist that ran before the hole — and `statement_that_refused`
            // has already cleared `temp` where the rendered text says one did.
            // Asking the VALUE instead was asking something a plain array, a
            // `Map` or a `Set` cannot answer.
            (None, Some(temp)) if !hoist.refused => wrap_release(&inner, temp),
            (None, _) => inner,
        };
        inner = format!("{}{}", hoist.declaration, wrapped);
    }
    if after.trim().is_empty() {
        return inner;
    }
    format!(
        "try {{\n{}}} finally {{\n{}}}\n",
        crate::body::indent(&inner),
        crate::body::indent(after)
    )
}

/// Release `temp` however `body` is left.
pub fn wrap_release(body: &str, temp: &str) -> String {
    let release = format!("dropOwned({});\n", temp);
    if body.trim().is_empty() {
        return release;
    }
    format!(
        "try {{\n{}}} finally {{\n{}}}\n",
        crate::body::indent(body),
        crate::body::indent(&release)
    )
}

/// Release `temp` however `body` is left, unless the runtime says somebody else
/// already owns it or has released it.
pub fn wrap_guarded(body: &str, temp: &str) -> String {
    let release = guarded_release(temp);
    if body.trim().is_empty() {
        return release;
    }
    format!(
        "try {{\n{}}} finally {{\n{}}}\n",
        crate::body::indent(body),
        crate::body::indent(&release)
    )
}

/// `dropOwned(x)`, asked of the runtime first.
///
/// The cast is load-bearing: a `?` on an `Option<T>` leaves the PAYLOAD in the
/// temporary, and `T` is whatever the source said — a number has no `isMoved`.
/// Reading it off an untyped view answers `undefined`, which is "nobody has
/// taken it", and `dropOwned` lets a primitive go.
///
/// R13(a), the LIMIT this form has, stated rather than left to be rediscovered:
/// `isMoved` is only ever true of a value the runtime marked, and `markMoved`
/// is protected on `AkObject` — only base's own wrappers call it. A value moved
/// into a plain array, a `Map`, or a field of a user struct is not marked, and
/// the port writes `Vec`, `HashMap` and `HashSet` as exactly those. So this
/// form answers "nobody has taken it" for every such value, and asking it about
/// one is asking a question it cannot answer. S1 was that: a `Vec<Token>` handed
/// to a consuming call by an earlier `?` was dropped a second time.
///
/// It is therefore kept to the case it was written for — a value the port knows
/// is one of base's own wrappers, which is the `?`'s `Result` and nothing else.
/// The two remaining callers that pass an arbitrary value are named where they
/// stand: `hoisted`'s `released_if_unreached` (N3's lifted argument) and
/// `hoisted_when_refused`'s `?` temporary on the `Option` side. Both owe the
/// same move to a lexical flag that `released_after_a_refusal` has already
/// made; neither has a reproduction on the corpus today.
pub fn guarded_release(name: &str) -> String {
    format!(
        "if ({name} != null && !({name} as any).isMoved && !({name} as any).isDropped) dropOwned({name});\n"
    )
}

/// Can the text this wrapper stands over LEAVE before it reads the wrapper?
///
/// R0(3): a `?`'s wrapper is consumed by the `unwrap` that stands in the
/// statement's own text, and that is a complete answer only where the text
/// reaches the `unwrap`. `Ok((make(a)?, make(b)?))` reaches the first wrapper's
/// `unwrap` only if the second `?` succeeds; when it returns `Err`, or when the
/// call three frames down throws, the first wrapper still holds its `Ok`
/// payload and nobody releases it. Rust drops that temporary on both paths.
///
/// So the question is asked of the emitted text, which is the thing that
/// actually decides it: what runs before the wrapper's first mention is
/// whatever is textually COMPLETE before it, and the only complete thing in
/// emitted output that can leave is a call that has already returned or a jump.
/// A finished call shows a `)`; a `throw` shows itself. `f(g(), _r0.unwrap())`
/// has `g()` before the mention and needs the release;
/// `Result.Ok(checkedMul(_r0.unwrap(), 2))` has only two calls still waiting
/// for their arguments, one of which IS the mention, so nothing has run yet and
/// no release is written. Neither has `const t = _r0.unwrap();`, which is what
/// a plain `let x = f()?;` becomes — writing one there wrapped every `?` in the
/// corpus in a `try`/`finally` that provably does nothing (17,977 emitted
/// lines, against 936 for this rule).
///
/// What this does NOT count is a property read that throws because the value
/// under it is `undefined`. Rust has no such step: reading a field cannot
/// panic, so an emitted read that throws is the port already being wrong about
/// a type, and the release above it would not make that right.
///
/// A wrapper the text never mentions is never consumed, so it is released.
fn may_leave_before_reading(body: &str, temp: &str) -> bool {
    let Some(at) = body.find(temp) else { return true };
    let before = &body[..at];
    before.contains(')') || before.contains("throw")
}

pub fn hoisted(body: &str, hoists: &[Hoist]) -> String {
    // Every lift that owes a release is consumed by the same call — the one the
    // statement's own text writes — so all their flags are set in one place,
    // immediately above that text and below every declaration the statement
    // lifted. That is where O6 already puts a local's move flag, and for the
    // same reason: an argument that throws must not leave a flag saying the
    // callee has the value.
    let sets: String = hoists
        .iter()
        .filter_map(|hoist| hoist.flag.as_ref())
        .map(|flag| format!("{} = true;\n", flag))
        .collect();
    let mut inner = format!("{}{}", sets, body);
    for hoist in hoists.iter().rev() {
        let wrapped = match (&hoist.owned, &hoist.temp) {
            (Some(owned), _) => wrap(&inner, owned),
            (None, Some(temp)) if hoist.flag.is_some() => wrap_flagged(&inner, temp, hoist),
            (None, Some(temp)) if hoist.wrapper && may_leave_before_reading(&inner, temp) => {
                wrap_guarded(&inner, temp)
            }
            (None, Some(temp)) if hoist.released_if_unreached => wrap_guarded(&inner, temp),
            (None, _) => inner,
        };
        inner = format!("{}{}", hoist.declaration, wrapped);
    }
    inner
}

/// Release `temp` however `body` is left, unless this frame's flag says the
/// call it was lifted for took it.
fn wrap_flagged(body: &str, temp: &str, hoist: &Hoist) -> String {
    let flag = hoist.flag.as_ref().expect("the caller matched on it");
    let release = format!("if (!{}) dropOwned({});\n", flag, temp);
    if body.trim().is_empty() {
        return release;
    }
    format!(
        "try {{\n{}}} finally {{\n{}}}\n",
        crate::body::indent(body),
        crate::body::indent(&release)
    )
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
