//! The R12 hole: what an emitted file carries where the port has no lowering
//! for a Rust shape, and the count of how many have been written.
//!
//! One spelling in one place, so a hole is greppable in emitted output and the
//! harness can hold a ledger of them — and so that "this body refused a shape"
//! is answered by the LOWERING rather than by searching the rendered text for
//! `unsupported(`, which made the emitter's own output an input.

use super::quoted;

/// Indent each line by 2 spaces
/// The text of an R12 hole: what an emitted file carries where the port has no
/// lowering for a Rust shape.
///
/// One spelling, in one place, so a hole is greppable in emitted output and the
/// harness can hold a ledger of them. `unsupported` answers `never`, so this
/// stands wherever the expression it replaces stood.
pub fn hole_text(what: &str) -> String {
    HOLES_WRITTEN.with(|n| n.set(n.get() + 1));
    if INSIDE_A_CALLABLE.with(|d| d.get()) == 0 {
        HOLES_IN_THE_OPEN.with(|n| n.set(n.get() + 1));
    }
    format!("unsupported({})", quoted(what))
}

/// Lower a CALLABLE's body: every hole written inside it is a hole the
/// statement's own evaluation did not reach.
///
/// W3/X6: which of the two a hole is decides what a refusing statement owes,
/// and it was answered by looking for a `=>` before the first `unsupported(`
/// in the rendered statement. That is a per-statement answer to a per-value
/// question, and it is not even a reliable one: `'=>'.length` in a string
/// suppressed the cleanup of a statement that really had refused, and a closure
/// argument standing beside an unrelated hole hid it. The LOWERING knows which
/// it is, because it knows when it is inside a callable's body.
pub fn inside_a_callable<R>(lower: impl FnOnce() -> R) -> R {
    INSIDE_A_CALLABLE.with(|d| d.set(d.get() + 1));
    let answer = lower();
    INSIDE_A_CALLABLE.with(|d| d.set(d.get() - 1));
    answer
}

/// How many holes have been written where the statement's OWN evaluation would
/// have reached them — the count a caller takes a delta of.
pub fn holes_in_the_open() -> usize {
    HOLES_IN_THE_OPEN.with(|n| n.get())
}

thread_local! {
    /// How many holes have been written since the process started.
    ///
    /// I1: "this body carries a hole" is the LOWERING's answer, and `hole_text`
    /// is the one place a hole's text is made — so counting here is counting
    /// what was lowered. Read as a delta around one body's translation. The
    /// alternative, searching the rendered text for `unsupported(`, made the
    /// emitter's own output an input: a body that mentions those characters for
    /// any other reason is not a body that refused a shape.
    static HOLES_WRITTEN: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };

    /// How deep inside a CALLABLE's body the lowering currently is.
    static INSIDE_A_CALLABLE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };

    /// Of the holes written, how many stood in the open — not inside any
    /// callable's body.
    static HOLES_IN_THE_OPEN: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The running count, for a caller taking a delta around a translation.
pub fn holes_written() -> usize {
    HOLES_WRITTEN.with(|n| n.get())
}

/// Where this emitted TEXT carries an R12 hole, if it does.
///
/// W10: ONE scanner. The same `find("unsupported(")` was written out in
/// `body/blocks.rs`, `body/refusal.rs`, `match_expr/owing.rs` and
/// `ownership/locals.rs`, and a rule about what a hole aborts is only as good
/// as every copy of the question agreeing with the others.
///
/// This asks about a rendered BODY, which the module doc warns against — a body
/// that mentions these characters for another reason is not a body that
/// refused. It is the position the callers need (what stands before the throw
/// and what stands after it), not the fact that one was written, and that fact
/// is `holes_written`.
pub fn hole_at(text: &str) -> Option<usize> {
    text.find("unsupported(")
}

/// Does this emitted text carry one anywhere in it?
pub fn holds_a_hole(text: &str) -> bool {
    hole_at(text).is_some()
}

/// Did lowering this value WRITE a hole, and is the value that hole?
///
/// X3: the second question alone read the emitted characters, and a user
/// function named `unsupported` writes exactly those characters —
/// `fn unsupported(_: &str) -> Option<u32>` with `let n = unsupported("x")?;`
/// lost its null test and reached `checkedAdd(null, 1)` on a valid program.
/// The first question is the provenance the reviewers asked for and `hole_text`
/// is the only thing that can answer yes to it: a call to a user function of
/// that name makes no hole, so the counter does not move and the `?` keeps its
/// test whatever the text looks like.
///
/// `before` is `holes_written()` read immediately before the value was lowered.
pub fn lowered_a_hole(before: usize, value: &str) -> bool {
    holes_written() > before && value_is_a_hole(value)
}

/// Is this lowered VALUE a hole — the whole of it, not something with one
/// buried inside?
///
/// R1: `try_operand` used to read the global hole counter before and after
/// lowering the `?`'s operand, so a refusal ANYWHERE in the operand's subtree —
/// inside a closure the operand passes, in one branch of an `if`, in an
/// unrelated argument beside it — was read as "the operand IS a hole" and the
/// `?` lost its null/error test. `let v = a.or_else(|| xs.into_iter().next())?;`
/// then computed on `null` where Rust answers `None`, with nothing said.
///
/// The counter answers "did anything refuse", which is the question `blocks.rs`
/// asks about a whole statement and the right one there. The question HERE is
/// about one value, so it is asked of that value. Reading the text is what the
/// module doc warns against when the text is a rendered BODY — a body that
/// mentions these characters for another reason is not a body that refused. One
/// lowered value is not a body: it either is the string `hole_text` just made
/// or it is not, and the leading `unsupported('` carries the quote a call to a
/// user function of that name would not have in that position.
/// W9: and the port WRAPS a value where the position needs it wrapped — an
/// `await` in front, parentheses around — so the hole is looked for under those
/// two. Asked without them, `(await unsupported('..'))` under a `?` was not
/// recognised and the `?` wrote a wrapper test below a throw.
pub fn value_is_a_hole(value: &str) -> bool {
    let mut value = value.trim();
    loop {
        if let Some(inner) = value.strip_prefix("await ") {
            value = inner.trim();
            continue;
        }
        // Only where the parentheses are the WHOLE of it: `(a)(b)` opens and
        // closes twice, and peeling its outermost pair would read `a)(b` as
        // the value.
        if let Some(inner) = value.strip_prefix('(').and_then(|v| v.strip_suffix(')')) {
            if balanced(inner) {
                value = inner.trim();
                continue;
            }
        }
        break;
    }
    value.starts_with("unsupported('") && value.ends_with(')')
}

/// Does this text never close more parentheses than it has opened?
///
/// DD9: a parenthesis inside the QUOTED diagnostic is a character, not a
/// bracket. Counted as one, `(unsupported('`try_into` on `Vec<u8>` (ambiguous)`
/// ..'))` came back unbalanced, the wrapper was not peeled, the value was not
/// recognised as a hole, and a `?` above it wrote its wrapper handling below a
/// throw. No corpus diagnostic has an unbalanced parenthesis today, which is
/// why this was latent; the quotes are what decide, not the count.
fn balanced(text: &str) -> bool {
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for c in text.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, c) {
            (Some(_), '\\') => escaped = true,
            (Some(open), c) if c == open => quote = None,
            (Some(_), _) => {}
            (None, '\'') | (None, '"') | (None, '`') => quote = Some(c),
            (None, '(') => depth += 1,
            (None, ')') => depth -= 1,
            (None, _) => {}
        }
        if depth < 0 {
            return false;
        }
    }
    depth == 0 && quote.is_none()
}

#[cfg(test)]
mod tests {
    use super::{holes_written, lowered_a_hole, value_is_a_hole};
    use crate::testing::Fixture;

    /// X3: a user function named `unsupported` writes exactly the characters a
    /// hole is spelled with, and writes no hole. The `?` keeps its test.
    #[test]
    fn a_user_function_named_unsupported_is_not_a_hole() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub fn unsupported(_: &str) -> Option<u32> { Some(3) }\n\
             pub fn probe() -> Option<u32> {\n\
               let n = unsupported(\"nothing to see\")?;\n\
               Some(n + 1)\n\
             }",
        )]);
        let ts = f.translated_method("lib.rs", "probe");
        assert!(
            ts.contains("if (_r0 == null) return null;"),
            "the `?` on a valid program keeps the test its Option needs:\n{}",
            ts
        );
    }

    /// And the provenance is what says so: nothing was written, so the counter
    /// did not move, whatever the text looks like.
    #[test]
    fn the_counter_is_what_answers_the_question() {
        let before = holes_written();
        assert!(
            !lowered_a_hole(before, "unsupported('a user wrote this')"),
            "no hole was made while this value was lowered"
        );
    }

    /// W9: the port wraps a value where the position needs it wrapped, and the
    /// hole is still the value.
    #[test]
    fn a_hole_is_recognised_under_parentheses_and_an_await() {
        assert!(value_is_a_hole("unsupported('x')"));
        assert!(value_is_a_hole("(unsupported('x'))"));
        assert!(value_is_a_hole("(await unsupported('x'))"));
        assert!(value_is_a_hole("await (unsupported('x'))"));
        assert!(value_is_a_hole("  ((await unsupported('x')))  "));
    }

    /// And a value that merely CONTAINS one is not one: the parentheses have to
    /// be the whole of it.
    #[test]
    fn a_value_holding_a_hole_is_not_the_hole() {
        assert!(!value_is_a_hole("tally(unsupported('x'))"));
        assert!(!value_is_a_hole("(unsupported('x')) + (1)"));
        assert!(!value_is_a_hole("(a)(unsupported('x'))"));
    }

    /// The `?` on a PARENTHESISED hole writes no wrapper test below the throw.
    #[test]
    fn a_question_mark_on_a_wrapped_hole_writes_no_dead_test() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub fn probe(xs: Vec<u32>) -> Option<u32> {\n\
               let total = (xs.into_iter().map(|x| x)\
                 .collect::<std::collections::BinaryHeap<u32>>())?;\n\
               Some(total)\n\
             }",
        )]);
        let ts = f.translated_method("lib.rs", "probe");
        assert!(ts.contains("unsupported("), "the collect was expected to refuse:\n{}", ts);
        assert!(
            !ts.contains("isErr()"),
            "the hole throws where it stands, so nothing below it is reached:\n{}",
            ts
        );
    }

    /// DD9: a parenthesis inside the quoted diagnostic is a character.
    #[test]
    fn a_wrapped_hole_is_recognised_through_an_unbalanced_quote() {
        assert!(super::value_is_a_hole(
            "(unsupported('`try_into` is ambiguous (two impls) here'))"
        ));
        assert!(super::value_is_a_hole(
            "(await unsupported('a paren ) inside the message'))"
        ));
        // And a value that is not a hole is still not one.
        assert!(!super::value_is_a_hole("(unsupported(what))"));
    }
}
