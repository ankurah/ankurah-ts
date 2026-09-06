//! The run of `if`s a chain of arms becomes, and the branch a guard opens.
//!
//! For: three lowerings write the same thing — the links of a variant several
//! arms name, the arms of one side of a `Result` match against the payload it
//! read, and the if-chain a value match is. Each had a copy, with its own
//! spelling of "an arm with neither a test nor a guard", its own label and its
//! own rule for when the label is needed. They drifted: one took a label
//! unconditionally and left four unused ones in the emitted corpus, and only one
//! of the three knew what a guard owes when it throws. This is the one renderer;
//! the three callers differ in what they build a `Branch` out of and in what
//! they write where nothing matched.

use crate::body::{indent, BodyTranslator};

/// An arm's guard: the test, what evaluating it costs, and what the link owes
/// if it throws.
///
/// Rust evaluates a guard AFTER the pattern has matched and the names are
/// bound, and it drops the guard's own temporaries before either the arm's body
/// or the next arm runs. So the guard belongs inside the branch the variant test
/// opened — written outside it, `match w { Wrap::Held(t) if *cell.lock()… }`
/// locked the mutex before the dispatch, for `Wrap::Empty` as well.
pub(in crate::match_expr) struct Guard {
    /// The test itself, as an expression.
    pub test: String,
    /// The declarations the test lifted out of itself, which stand inside the
    /// branch with it and are released there.
    pub lifted: Vec<crate::ownership::Hoist>,
    /// What the link owes if the test THROWS: the pattern has already handed
    /// the payload over, the arm's own `finally` has not been entered, and the
    /// arm below never runs. Empty where the link owns nothing.
    ///
    /// It is ALWAYS empty for the value-match caller, and that is not an
    /// oversight: a value match writes the if-chain over a subject it never
    /// took apart, so a guard that throws there leaves nothing the block
    /// around it does not already release. The two consuming callers — a
    /// variant's links and a `Result` side's arms — are the ones that fill it.
    pub release: String,
}

/// The branch a guard opens, and what stands around it.
///
/// Both consuming chains — a variant's links and a `Result` side's arms — write
/// the same three pieces: the names the pattern bound, the guard tested against
/// them, and the body inside the `if` the guard opens.
pub(in crate::match_expr) fn guarded_branch(
    bindings: &str,
    guard: &Guard,
    inner: &str,
    t: &BodyTranslator,
) -> String {
    let Guard { test, lifted, release } = guard;
    if release.is_empty() {
        // Nothing is owed, so the test can stand in the `if` with whatever it
        // lifted settled in front of it.
        let (test, before) = t.settle_condition(test.clone(), lifted);
        return format!("{}{}if ({}) {{\n{}}}\n", bindings, before, test, indent(inner));
    }
    // The test is made in a `try` of its own, and NOT the body: the body has a
    // `finally` that releases the same names, and running both would release
    // them twice.
    //
    // The two fatal throws leave the `catch` before anything is released.
    // `port/ownership.md` says a `catch` rethrows an `OwnershipFatal` and an
    // `UnsupportedShape` first: releasing on the way out of one would drop a
    // value the registry has just said was already dropped, and the second
    // fatal would bury the first.
    let held = t.fresh_hoist("_g");
    format!(
        "{}let {};\ntry {{\n{}}} catch (_e) {{\n{}}}\nif ({}) {{\n{}}}\n",
        bindings,
        held,
        indent(&crate::ownership::hoisted(
            &format!("{} = {};\n", held, test),
            lifted
        )),
        indent(&format!(
            "if (_e instanceof OwnershipFatal || _e instanceof UnsupportedShape) throw _e;\n{}throw _e;\n",
            release
        )),
        held,
        indent(inner)
    )
}

/// One branch of a chain that tries its arms IN TURN.
///
/// For: three lowerings write the same run of `if`s — a variant's contested
/// links, a `Result` side's arms against the payload it read, and the if-chain a
/// value match is. Each had its own copy, with its own spelling of "an arm with
/// neither a test nor a guard", its own label, and its own rule for when the
/// label is needed; they drifted, and one of them took a label unconditionally
/// and left four unused ones in the corpus. This is the one renderer, and the
/// three callers differ only in what they build these out of and in what they
/// write where nothing matched.
pub(in crate::match_expr) struct Branch {
    /// The test this branch's own pattern makes, or nothing where it matches
    /// whatever reaches it.
    pub test: Option<String>,
    /// What the branch declares before its guard, because the guard reads it.
    pub bindings: String,
    pub guard: Option<Guard>,
    /// Everything that runs when both tests pass.
    pub block: String,
    /// Whether the block leaves by itself, so nothing written after it in the
    /// chain would run.
    pub leaves: bool,
}

impl Branch {
    /// A branch with neither a test nor a guard matches every value that
    /// reaches it, so Rust never reads past it and neither does the chain.
    pub(in crate::match_expr) fn unconditional(&self) -> bool {
        self.test.is_none() && self.guard.is_none()
    }
}

/// The branches tried in turn inside a labelled block, with `tail` written
/// where none of them ran.
///
/// `stem` names the label, so a reader can tell a variant's chain from a
/// `Result` side's at a glance. A label nothing jumps to is noise, so it is
/// taken only where some branch has to jump over what stands after it.
pub(in crate::match_expr) fn tried_in_turn(
    branches: &[Branch],
    tail: &str,
    stem: &str,
    t: &BodyTranslator,
) -> String {
    let needs_break = branches.iter().enumerate().any(|(at, branch)| {
        !branch.leaves && !branch.unconditional() && (!tail.is_empty() || at + 1 < branches.len())
    });
    let label = if needs_break { t.fresh_hoist(stem) } else { String::new() };
    let mut inner = String::new();
    for branch in branches {
        let leaving = if branch.leaves || branch.unconditional() || label.is_empty() {
            String::new()
        } else {
            format!("break {};\n", label)
        };
        let body = format!("{}{}", branch.block, leaving);
        let guarded = match &branch.guard {
            Some(guard) => guarded_branch(&branch.bindings, guard, &body, t),
            None => format!("{}{}", branch.bindings, body),
        };
        match &branch.test {
            Some(test) => inner.push_str(&format!("if ({}) {{\n{}}}\n", test, indent(&guarded))),
            // A pattern that matches every value still opens a block of its
            // own: the names it binds belong to this branch and to no branch
            // written after it.
            None => inner.push_str(&format!("{{\n{}}}\n", indent(&guarded))),
        }
    }
    inner.push_str(tail);
    if label.is_empty() {
        return inner;
    }
    format!("{}: {{\n{}}}\n", label, indent(&inner))
}

