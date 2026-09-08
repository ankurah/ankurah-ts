//! Where a hoisted value is DECLARED, and how it is released.
//!
//! A hoist's declaration has to stand before the text that names it, and the
//! value it declared has to be released however that text is left. Which of the
//! four shapes a hoist takes — released outright, released behind this frame's
//! own flag, released through the runtime's own mark, or not released at all —
//! is decided here, in one place, because the four differ only in what is known
//! about the value and about the text below it.

use super::{without_declaration, sets_the_flag, Hoist, Owned};

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
///
/// X8: an `await` is a third way out. `Ok((future.await, pass(token)?))` hoists
/// the `?` above the await, so the emitted text reaches the await holding the
/// wrapper — and a rejected promise leaves through it with nobody releasing
/// what the wrapper holds. It shows no `)` and no `throw`, so the rule said the
/// text could not leave.
///
/// All three are looked for OUTSIDE string literals, because the emitted text
/// carries the corpus's own strings and a `)` inside one is not a call that
/// returned.
fn may_leave_before_reading(body: &str, temp: &str, suspends: bool) -> bool {
    // W10: as a WHOLE identifier. A bare `find` made `_r1` a prefix of `_r12`,
    // so the wrapper's own first mention could be somebody else's name.
    let Some(at) = crate::body::refusal::mentions_at(body, temp) else { return true };
    let before = outside_strings(&body[..at]);
    // GG5: the SUSPENSION comes from the lowering, which knows which awaits run
    // while the wrapper is in hand. Counted by bracket depth in the rendered
    // text, `Result.Ok(await future + eat(_r0.unwrap()))` read as "the mention
    // stands inside the awaited operand" — the call around the mention leaves
    // an opening `(` — and a rejected promise left one `Result` and two
    // `Token`s with nobody.
    suspends || before.contains(')') || before.contains("throw")
}


/// The emitted text with every string literal's CONTENT taken out, so that a
/// rule about what the code does is not answered by what the code prints.
///
/// The port writes strings with single quotes, template literals with
/// backticks, and passes double-quoted text through from the corpus. A
/// backslash escapes the next character in all three.
fn outside_strings(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for c in text.chars() {
        match quote {
            Some(open) => {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == open {
                    quote = None;
                    out.push(c);
                }
            }
            None => {
                if matches!(c, '\'' | '"' | '`') {
                    quote = Some(c);
                }
                out.push(c);
            }
        }
    }
    out
}

/// Where every `await` this statement writes finishes SUSPENDING — the end of
/// its own operand — outside any closure.
///
/// X8/GG5/FF8: a `?` is hoisted ABOVE the statement, so its wrapper is live
/// from the top of it; an `await` that suspends after the hoist and before the
/// wrapper's first mention can reject and leave with nobody releasing what the
/// wrapper holds. What tells those apart is where the await's OPERAND ends
/// against where the `?` stands: `f.await + eat(pass(t)?)` finishes awaiting
/// before the `?`, and `step(pass(n)?).await` finishes after it, because its
/// operand is what the `?` helped to build. Counted as brackets in the rendered
/// text instead, a call around the wrapper's first mention read as "the mention
/// stands inside the awaited operand" and no guard was written at all (GG5).
///
/// A closure has its own prelude and its own hoists, so an `await` inside one
/// suspends while nothing of this statement is in hand.
pub fn suspensions_in(stmt: &syn::Stmt) -> Vec<(usize, usize)> {
    let mut walk = Awaits::default();
    syn::visit::Visit::visit_stmt(&mut walk, stmt);
    walk.found
}

/// Does one of those suspensions happen BEFORE this `?` stands?
pub fn suspends_before(at: &[(usize, usize)], question: &syn::Expr) -> bool {
    let start = syn::spanned::Spanned::span(question).start();
    at.iter().any(|end| *end < (start.line, start.column))
}

#[derive(Default)]
struct Awaits {
    found: Vec<(usize, usize)>,
}

impl syn::visit::Visit<'_> for Awaits {
    fn visit_expr(&mut self, expr: &syn::Expr) {
        if matches!(expr, syn::Expr::Closure(_)) {
            return;
        }
        if let syn::Expr::Await(await_) = expr {
            let end = syn::spanned::Spanned::span(&*await_.base).end();
            self.found.push((end.line, end.column));
        }
        syn::visit::visit_expr(self, expr);
    }
}

pub fn hoisted(body: &str, hoists: &[Hoist]) -> String {
    // W1/X1: a lift's flag says "the call this was lifted for took it", and
    // that is a claim about text the port actually WROTE. Where the call is a
    // hole — `top_k` in `storage-indexeddb/collection.ts`, which the port
    // refused — nothing below the lift names the temporary at all, so the flag
    // is a lie the `finally` believes and the clone is released by nobody.
    // Asked of the temporary rather than of the hole, because a transfer that
    // is not written is a transfer that does not happen however the text
    // failed to be written.
    let below_of = |index: usize| -> String {
        hoists[index + 1..]
            .iter()
            .map(|hoist| hoist.declaration.as_str())
            .chain(std::iter::once(body))
            .collect()
    };
    let takes = |index: usize| -> bool {
        let Some(temp) = hoists[index].temp.as_deref() else { return false };
        crate::body::refusal::mentions(&below_of(index), temp)
    };
    // Which hoists actually READ their flag, so that the `let` and the `= true`
    // are written exactly where the `finally` tests one. W14's payload asks one
    // question more than a lift does: a payload the text cannot leave before
    // reading is consumed on every path, and wrapping it would put a `try` and
    // a `finally` around every `?` on an `Option` in the corpus.
    let uses_flag = |index: usize| -> bool {
        let hoist = &hoists[index];
        let Some(temp) = hoist.temp.as_deref() else { return false };
        hoist.flag.is_some()
            && takes(index)
            && (!hoist.payload || may_leave_before_reading(&below_of(index), temp, hoist.suspends))
    };
    // Every lift that owes a release is consumed by the same call — the one the
    // statement's own text writes — so all their flags are set in one place,
    // immediately above that text and below every declaration the statement
    // lifted. That is where O6 already puts a local's move flag, and for the
    // same reason: an argument that throws must not leave a flag saying the
    // callee has the value.
    let sets: String = hoists
        .iter()
        .enumerate()
        .filter(|(index, _)| uses_flag(*index))
        .filter_map(|(_, hoist)| hoist.flag.as_ref())
        .map(|flag| format!("{} = true;\n", flag))
        .collect();
    let mut inner = format!("{}{}", sets, body);
    for (index, hoist) in hoists.iter().enumerate().rev() {
        let wrapped = match (&hoist.owned, &hoist.temp) {
            (Some(owned), _) => wrap(&inner, owned),
            (None, Some(temp)) if uses_flag(index) => wrap_flagged(&inner, temp, hoist),
            // W1/X2: nothing below takes it, so the release is unconditional
            // and the flag — if this lift declared one — is a `let` nothing
            // assigns, which E15 already strikes for a block's own locals.
            (None, Some(temp)) if !takes(index) && (hoist.flag.is_some() || hoist.droppable) => {
                wrap_release(&inner, temp)
            }
            (None, Some(temp)) if hoist.wrapper && may_leave_before_reading(&inner, temp, hoist.suspends) => {
                wrap_guarded(&inner, temp)
            }
            (None, Some(temp)) if hoist.released_if_unreached => wrap_guarded(&inner, temp),
            (None, _) => inner,
        };
        let declaration = match &hoist.flag {
            Some(flag) if !uses_flag(index) => without_declaration(&hoist.declaration, flag),
            _ => hoist.declaration.clone(),
        };
        // U3: the flag stands immediately above the declaration whose call
        // performs the transfer, which is below everything the statement
        // lifted out of itself.
        inner = format!("{}{}{}", hoist.sets, declaration, wrapped);
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

