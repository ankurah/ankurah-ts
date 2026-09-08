//! Where a `?` puts the value it tested, and who releases it.
//!
//! The `?` lowering lifts a temporary out of the statement that wrote it: the
//! `Result` it tests, or — for an `Option` — the payload itself. These tests
//! say what stands around that temporary. They were split out of `tests.rs`,
//! which was over the 600-line rule.

use super::tests::body;

// ── Result values, not throws ─────────────────────────────────────────

#[test]
fn a_question_mark_returns_the_error_and_consumes_both_wrappers() {
    let ts = body(
        "pub fn g() -> Result<u32, String> { Ok(1) }\n\
         pub fn f() -> Result<u32, String> { let n = g()?; Ok(n + 1) }",
        "f",
    );
    assert_eq!(
        ts.trim(),
        "const _r0 = g();\n\
         if (_r0.isErr()) return Result.Err(_r0.unwrapErr());\n\
         const n = _r0.unwrap();\n\
         return Result.Ok(checkedAdd(n, 1, 'u32'));",
        "{}",
        ts
    );
}

#[test]
fn a_question_mark_in_statement_position_releases_the_ok_wrapper() {
    let ts = body(
        "pub fn g() -> Result<u32, String> { Ok(1) }\n\
         pub fn f() -> Result<u32, String> { g()?; Ok(0) }",
        "f",
    );
    assert!(
        ts.contains("_r0.drop();"),
        "the Ok Result nobody bound is released rather than leaked:\n{}",
        ts
    );
}

#[test]
fn a_question_mark_inside_an_expression_is_lifted_out_of_it() {
    let ts = body(
        "pub fn g() -> Result<u32, String> { Ok(1) }\n\
         pub fn f() -> Result<u32, String> { Ok(g()? + 1) }",
        "f",
    );
    assert_eq!(
        ts.trim(),
        "const _r0 = g();\n\
         if (_r0.isErr()) return Result.Err(_r0.unwrapErr());\n\
         return Result.Ok(checkedAdd(_r0.unwrap(), 1, 'u32'));",
        "{}",
        ts
    );
}

#[test]
fn unwrap_or_on_a_result_calls_the_runtime_rather_than_the_null_coalesce() {
    let ts = body(
        "pub fn g() -> Result<u32, String> { Ok(1) }\n\
         pub fn f() -> u32 { g().unwrap_or(0) }",
        "f",
    );
    assert!(
        ts.contains("unwrapOr(0)"),
        "`?? 0` reads a Result object as truthy and always takes it:\n{}",
        ts
    );
    assert!(!ts.contains("??"), "{}", ts);
}

#[test]
fn unwrap_or_on_an_option_stays_the_null_coalesce() {
    let ts = body(
        "pub fn g() -> Option<u32> { Some(1) }\n\
         pub fn f() -> u32 { g().unwrap_or(0) }",
        "f",
    );
    assert!(ts.contains("?? 0"), "{}", ts);
}

// ── Where a temporary lives ───────────────────────────────────────────

#[test]
fn a_condition_temporary_is_released_before_the_body_runs() {
    let ts = body(
        "pub fn f(h: &Held) -> u32 { if *h.cell.lock().unwrap() == 0 { 1 } else { 2 } }",
        "f",
    );
    let released = ts.find("_t0.drop()").expect("the guard is released");
    let branch = ts.find("if (_c1)").expect("the test stands on its own");
    assert!(
        released < branch,
        "the lock is released before the branch is taken:\n{}",
        ts
    );
}

#[test]
fn a_while_condition_is_evaluated_again_each_turn() {
    let ts = body(
        "pub fn f(h: &Held) -> u32 { let mut n = 0; while *h.cell.lock().unwrap() > n { n += 1; } n }",
        "f",
    );
    assert!(ts.contains("for (;;) {"), "the test moves inside the loop:\n{}", ts);
    let loop_at = ts.find("for (;;)").expect("the loop");
    let lock_at = ts.find("h.cell.lock()").expect("the lock");
    assert!(lock_at > loop_at, "the lock is taken each turn:\n{}", ts);
    assert!(ts.contains("_t0.drop();"), "and released each turn:\n{}", ts);
}

#[test]
fn a_borrowed_temporary_in_an_argument_is_released_after_the_call() {
    let ts = body("pub fn f() -> u32 { look(&Owned { n: 1 }) }", "f");
    assert_eq!(
        ts.trim(),
        "const _t0 = new Owned(1);\n\
         try {\n  \
           return look(_t0);\n\
         } finally {\n  \
           _t0.drop();\n\
         }",
        "{}",
        ts
    );
}


// ── A wrapper the statement left behind (R0(3)) ───────────────────────

#[test]
fn a_second_question_mark_releases_the_first_ones_wrapper() {
    let ts = body(
        "pub fn f() -> Result<(Owned, Owned), Oops> { Ok((fallible(1)?, fallible(2)?)) }",
        "f",
    );
    assert!(
        ts.contains("if (_r0 != null && !(_r0 as any).isMoved && !(_r0 as any).isDropped) dropOwned(_r0);"),
        "the first `?`'s wrapper is released however the statement is left, because \
         the second one can return or throw before the `unwrap` that consumes it:\n{}",
        ts
    );
}

#[test]
fn a_lone_question_mark_whose_unwrap_stands_alone_writes_no_release() {
    let ts = body(
        "pub fn f() -> Result<u32, Oops> { let o = fallible(1)?; Ok(o.n) }",
        "f",
    );
    assert!(
        !ts.contains("dropOwned(_r0)"),
        "nothing runs between the wrapper's declaration and the `unwrap` that \
         consumes it, so no release is written:\n{}",
        ts
    );
}

#[test]
fn a_completed_call_before_the_unwrap_earns_the_release() {
    let ts = body(
        "pub fn f() -> Result<u32, Oops> { Ok(take(fallible(1)?) + look(&Owned { n: 2 })) }",
        "f",
    );
    assert!(
        ts.contains("dropOwned(_r0)"),
        "`look(..)` finishes before the wrapper is read, and any finished call \
         could have thrown:\n{}",
        ts
    );
}

#[test]
fn an_option_question_mark_writes_no_guarded_release() {
    let ts = body(
        "pub fn f(a: bool, b: bool) -> Option<u32> { Some(maybe(a)?.n + maybe(b)?.n) }",
        "f",
    );
    assert!(
        !ts.contains("isMoved"),
        "a `?` on an `Option` leaves the PAYLOAD in the temporary, which may be a \
         plain array or `Map` carrying no move mark; S1 retires that guard rather \
         than adding a use of it:\n{}",
        ts
    );
}

// ── U3: the flag stands above the transfer, not above the prelude ─────

/// A local handed to a `?`'d call has its move flag set immediately above the
/// call that takes it — below every argument the statement lifted so that the
/// flag could stand below them.
///
/// U3: the flag used to stand above the whole prelude, which is above those
/// lifts. `self.0.query(R::Model::collection(), args)?` in `core/context.ts`
/// marked `args` handed over and THEN called `R::Model::collection()`, so that
/// call's throw path left `args` with a flag saying somebody else owned it and
/// nobody released it.
#[test]
fn a_question_marks_flag_stands_below_the_arguments_lifted_above_it() {
    let ts = body(
        "pub fn build() -> Owned { Owned { n: 1 } }\n\
         pub fn eat(a: Owned, b: Owned) -> Result<u32, Oops> { Ok(a.n + b.n) }\n\
         pub fn f() -> Result<u32, Oops> {\n\
           let held = Owned { n: 2 };\n\
           let gate = fallible(0)?;\n\
           let n = eat(build(), held)?;\n\
           Ok(n + gate.n)\n\
         }",
        "f",
    );
    let lift = ts.find("= build();").expect(&ts);
    let flag = ts.find("_moved0 = true;").expect(&ts);
    let call = ts.find("= eat(").expect(&ts);
    assert!(
        lift < flag && flag < call,
        "the lifted argument stands above the flag and the flag above the call:\n{}",
        ts
    );
}

/// Two `?` operands in one statement each set their OWN transfer's flag, at
/// their own hoist. Setting both above the first one leaves the second local
/// flagged while the first call can still throw.
#[test]
fn each_question_mark_sets_only_the_flags_its_own_operand_hands_away() {
    let ts = body(
        "pub fn eat(o: Owned) -> Result<u32, Oops> { Ok(o.n) }\n\
         pub fn f() -> Result<u32, Oops> {\n\
           let first = Owned { n: 1 };\n\
           let second = Owned { n: 2 };\n\
           let gate = fallible(0)?;\n\
           let n = eat(first)? + eat(second)?;\n\
           Ok(n + gate.n)\n\
         }",
        "f",
    );
    let first_flag = ts.find("_moved0 = true;").expect(&ts);
    let first_call = ts.find("= eat(first);").expect(&ts);
    let second_flag = ts.find("_moved1 = true;").expect(&ts);
    let second_call = ts.find("= eat(second);").expect(&ts);
    assert!(
        first_flag < first_call && first_call < second_flag && second_flag < second_call,
        "the second local is flagged only once the first call has returned:\n{}",
        ts
    );
}

/// A `?` whose operand hands a local away still sets the flag ABOVE the hoist,
/// because the statement's own text — where every other flag stands — is never
/// reached on the error path.
#[test]
fn a_question_marks_flag_still_stands_above_the_hoist_that_consumes() {
    let ts = body(
        "pub fn eat(o: Owned) -> Result<u32, Oops> { Ok(o.n) }\n\
         pub fn f() -> Result<u32, Oops> {\n\
           let held = Owned { n: 1 };\n\
           let gate = fallible(0)?;\n\
           let n = eat(held)?;\n\
           Ok(n + gate.n)\n\
         }",
        "f",
    );
    let flag = ts.find("_moved0 = true;").expect(&ts);
    let call = ts.find("= eat(held);").expect(&ts);
    assert!(flag < call, "the flag is set before the call that can leave:\n{}", ts);
}

/// X8 and W14: what a `?` leaves behind on the paths the statement can take out
/// from under it.
mod leaving_before_the_read {
    use crate::testing::Fixture;

    const SETUP: &str = "\
pub struct Token { pub n: u32 }\n\
impl Drop for Token { fn drop(&mut self) {} }\n\
pub fn passr(t: Token) -> Result<Token, u32> { Ok(t) }\n\
pub fn eat(v: Vec<Token>) -> u32 { v.len() as u32 }\n\
pub fn maybe_vec(flag: bool) -> Option<Vec<Token>> { if flag { Some(Vec::new()) } else { None } }\n\
";

    fn emitted(rust: &str) -> String {
        let mut fixture = Fixture::build(&[("lib.rs", &format!("{}{}", SETUP, rust))]);
        fixture.emitted("lib.rs")
    }

    /// X8: an `await` is a third way out of a statement. The `?` above it is
    /// hoisted, so the text reaches the await holding the wrapper, and a
    /// rejected promise leaves through it with nobody releasing what the
    /// wrapper holds.
    #[test]
    fn an_await_that_has_run_leaves_the_wrapper_owed() {
        let ts = emitted(
            "pub async fn awaited(t: Token, f: impl std::future::Future<Output = u32>)\n\
             -> Result<(u32, Token), u32> { Ok((f.await, passr(t)?)) }",
        );
        let at = ts.find("function awaited").expect("the function is emitted");
        assert!(
            ts[at..].contains("finally"),
            "the wrapper is released however the text is left:\n{}",
            ts
        );
    }

    /// And an `await` the mention stands INSIDE has not run: it awaits what the
    /// mention helps to build, so the mention is evaluated first.
    #[test]
    fn an_await_whose_operand_holds_the_mention_owes_nothing() {
        let ts = emitted(
            "pub fn passn(n: u32) -> Result<u32, u32> { Ok(n) }\n\
             pub async fn step(n: u32) -> u32 { n }\n\
             pub async fn awaited(n: u32) -> Result<u32, u32> { Ok(step(passn(n)?).await) }",
        );
        let at = ts.find("function awaited").expect("the function is emitted");
        let awaited = &ts[at..];
        assert!(awaited.contains("await step(_r0.unwrap())"), "the await is written:\n{}", ts);
        assert!(!awaited.contains("finally"), "and nothing can leave before the read:\n{}", ts);
    }

    /// GG5: the wrapper's first mention inside a LATER CALL. The rule counted
    /// brackets in the rendered text, and the call around the mention leaves an
    /// opening `(` — so `Result.Ok(await f + eat(_r0.unwrap()))` read as "the
    /// mention stands inside the awaited operand" and no guard was written,
    /// leaving one `Result` and two `Token`s with nobody under a rejection.
    #[test]
    fn an_await_that_suspends_before_a_later_call_owes_the_wrapper() {
        let ts = emitted(
            "pub fn eat(t: Token) -> u32 { t.n }\n\
             pub async fn step(n: u32) -> u32 { n }\n\
             pub async fn after(t: Token, n: u32) -> Result<u32, u32> \
             { Ok(step(n).await + eat(passr(t)?)) }",
        );
        let at = ts.find("function after").expect("the function is emitted");
        assert!(ts[at..].contains("finally"), "the wrapper is released:\n{}", ts);
    }

    /// W14: a `?` on an `Option` writes no wrapper — the temporary IS the
    /// payload — and a SECOND `?` in the same statement returns with the first
    /// one's payload still in hand. Rust drops that temporary on the way out.
    #[test]
    fn an_option_payload_is_released_when_a_later_question_leaves() {
        let ts = emitted(
            "pub fn two(a: bool, b: bool) -> Option<u32> {\n\
               Some(eat(maybe_vec(a)?) + eat(maybe_vec(b)?))\n\
             }",
        );
        assert!(
            ts.contains("if (!_r0_kept) dropOwned(_r0);"),
            "the first payload is released from this frame's own flag:\n{}",
            ts
        );
        assert!(
            !ts.contains("isMoved"),
            "and never from the runtime guard, which an array does not answer (S1):\n{}",
            ts
        );
    }

    /// And a `?` the text cannot leave before reading writes neither the flag
    /// nor the release: wrapping those would put a `try` and a `finally` around
    /// every `?` on an `Option` in the corpus.
    #[test]
    fn an_option_payload_the_text_reaches_is_not_wrapped() {
        let ts = emitted(
            "pub fn one(a: bool) -> Option<u32> {\n\
               let v = maybe_vec(a)?;\n\
               Some(eat(v))\n\
             }",
        );
        assert!(!ts.contains("_kept"), "no flag is declared:\n{}", ts);
    }
}
