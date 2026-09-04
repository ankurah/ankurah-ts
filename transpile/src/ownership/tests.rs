//! What the emitted TypeScript says about ownership, rule by rule.
//!
//! Each test hands the transpiler a few lines of Rust and reads the emitted
//! body: where the releases are, which locals get one, and which do not.

use crate::testing::Fixture;

/// A crate whose types cover the cases the rules turn on: one with drop glue,
/// one `Copy`, one with `impl Drop`, and a container that hands out a guard.
const PRELUDE: &str = "\
use std::sync::Mutex;\n\
pub struct Owned { pub n: u32 }\n\
#[derive(Clone, Copy)]\n\
pub struct Id(pub u32);\n\
pub struct Held { pub cell: Mutex<u32> }\n\
pub struct Pair { pub one: Owned, pub two: Owned }\n\
pub struct Oops;\n\
pub enum Choice { One(Owned), Two(Owned) }\n\
pub fn take(o: Owned) -> u32 { o.n }\n\
pub fn look(o: &Owned) -> u32 { o.n }\n\
pub fn fallible(n: u32) -> Result<Owned, Oops> { Ok(Owned { n }) }\n\
pub fn maybe(f: bool) -> Option<Owned> { None }\n\
";

fn body(rust: &str, method: &str) -> String {
    let mut fixture = Fixture::build(&[("lib.rs", &format!("{}{}", PRELUDE, rust))]);
    fixture.translated_method("lib.rs", method)
}

// ── Block-owned drops ─────────────────────────────────────────────────

#[test]
fn a_block_releases_what_it_owns_in_a_finally() {
    let ts = body(
        "pub fn f() -> u32 { let a = Owned { n: 1 }; look(&a) }",
        "f",
    );
    assert_eq!(
        ts.trim(),
        "const a = new Owned(1);\n\
         try {\n  \
           return look(a);\n\
         } finally {\n  \
           a.drop();\n\
         }",
        "{}",
        ts
    );
}

#[test]
fn locals_are_released_in_reverse_declaration_order() {
    let ts = body(
        "pub fn f() -> u32 { let a = Owned { n: 1 }; let b = Owned { n: 2 }; look(&a) + look(&b) }",
        "f",
    );
    let first = ts.find("a.drop()").expect("a is released");
    let second = ts.find("b.drop()").expect("b is released");
    assert!(
        second < first,
        "b was declared second and must be released first:\n{}",
        ts
    );
}

#[test]
fn an_early_return_leaves_through_the_finally() {
    let ts = body(
        "pub fn f(stop: bool) -> u32 { let a = Owned { n: 1 }; if stop { return 0; } look(&a) }",
        "f",
    );
    assert!(ts.contains("return 0;"), "{}", ts);
    assert!(ts.contains("} finally {\n  a.drop();"), "{}", ts);
    assert_eq!(ts.matches("a.drop()").count(), 1, "released once only:\n{}", ts);
}

#[test]
fn a_copy_type_is_never_released() {
    let ts = body("pub fn f() -> u32 { let id = Id(7); id.0 }", "f");
    assert!(!ts.contains("drop"), "a Copy type has no drop glue:\n{}", ts);
    assert!(!ts.contains("try {"), "and so the block owns nothing:\n{}", ts);
}

#[test]
fn a_primitive_local_is_never_released() {
    let ts = body("pub fn f() -> u32 { let n = 3u32; n + 1 }", "f");
    assert!(!ts.contains("drop"), "{}", ts);
}

// ── Moves ─────────────────────────────────────────────────────────────

#[test]
fn a_local_passed_by_value_is_not_released() {
    let ts = body("pub fn f() -> u32 { let a = Owned { n: 1 }; take(a) }", "f");
    assert_eq!(ts.trim(), "const a = new Owned(1);\nreturn take(a);", "{}", ts);
}

#[test]
fn a_local_passed_by_reference_is_still_released() {
    let ts = body("pub fn f() -> u32 { let a = Owned { n: 1 }; look(&a) }", "f");
    assert!(ts.contains("a.drop();"), "{}", ts);
}

#[test]
fn a_local_moved_into_a_struct_literal_is_not_released() {
    let ts = body(
        "pub struct Pair { pub one: Owned }\n\
         pub fn f() -> Pair { let a = Owned { n: 1 }; Pair { one: a } }",
        "f",
    );
    assert!(!ts.contains("a.drop()"), "{}", ts);
}

#[test]
fn a_returned_local_is_not_released() {
    let ts = body("pub fn f() -> Owned { let a = Owned { n: 1 }; a }", "f");
    assert_eq!(ts.trim(), "const a = new Owned(1);\nreturn a;", "{}", ts);
}

#[test]
fn a_move_closure_that_captures_a_droppable_becomes_an_owned_closure() {
    let mut fixture = Fixture::build(&[(
        "lib.rs",
        &format!(
            "{}pub fn f() -> Box<dyn Fn() -> u32> {{ \
               let a = Owned {{ n: 1 }}; Box::new(move || a.n) \
             }}",
            PRELUDE
        ),
    )]);
    let ts = fixture.translated_method("lib.rs", "f");
    assert!(!ts.contains("a.drop()"), "the closure owns it now:\n{}", ts);
    assert!(
        ts.contains("new OwnedClosure([a], () => a.n)"),
        "the capture is listed beside the body:\n{}",
        ts
    );
    assert!(
        fixture
            .messages()
            .iter()
            .any(|m| m.contains("this closure owns `a`")
                && m.contains("cannot see this call site")),
        "a call site the emitter did not rewrite is reported: {:?}",
        fixture.messages()
    );
}

#[test]
fn a_conditional_move_is_released_behind_a_drop_flag() {
    let ts = body(
        "pub fn f(go: bool) -> u32 { \
           let a = Owned { n: 1 }; if go { return take(a); } 0 \
         }",
        "f",
    );
    assert!(ts.contains("let _moved0 = false;"), "{}", ts);
    assert!(ts.contains("_moved0 = true;"), "{}", ts);
    assert!(ts.contains("if (!_moved0) a.drop();"), "{}", ts);
}

#[test]
fn a_match_arm_that_hands_a_local_away_sets_its_flag() {
    let ts = body(
        "pub enum Choice { Keep, Hand }\n\
         pub fn f(c: Choice) -> u32 { \
           let a = Owned { n: 1 }; \
           match c { Choice::Keep => look(&a), Choice::Hand => take(a) } \
         }",
        "f",
    );
    // An arm is an arrow function, so the flag is set inside it; JavaScript
    // closes over the variable, so the `finally` reads what the arm wrote.
    assert!(
        ts.contains("Hand: () => {\n        _moved0 = true;"),
        "{}",
        ts
    );
    assert!(ts.contains("if (!_moved0) a.drop();"), "{}", ts);
}

#[test]
fn a_self_taking_method_moves_its_receiver() {
    let ts = body(
        "impl Owned { pub fn into_n(self) -> u32 { self.n } }\n\
         pub fn f() -> u32 { let a = Owned { n: 1 }; a.into_n() }",
        "f",
    );
    assert!(!ts.contains("a.drop()"), "`into_n` took it:\n{}", ts);
}

#[test]
fn drop_of_a_local_releases_it_once() {
    let ts = body(
        "pub fn f() -> u32 { let a = Owned { n: 1 }; drop(a); 0 }",
        "f",
    );
    assert_eq!(ts.matches("a.drop()").count(), 1, "{}", ts);
    assert!(!ts.contains("finally"), "nothing is left to release:\n{}", ts);
}

// ── Statement-scoped temporaries ──────────────────────────────────────

#[test]
fn a_guard_the_statement_produced_is_released_at_its_end_and_in_the_finally() {
    let ts = body(
        "impl Held { pub fn f(&self) -> u32 { let n = *self.cell.lock().unwrap(); n + 1 } }",
        "f",
    );
    assert!(ts.contains("const _t0 = this.cell.lock();"), "{}", ts);
    assert!(ts.contains("const n = _t0.value;"), "{}", ts);
    assert_eq!(
        ts.matches("_t0.drop();").count(),
        2,
        "released at the end of its statement and again in the finally, \
         which a guard's idempotent drop is there for:\n{}",
        ts
    );
}

#[test]
fn a_lock_call_needs_no_unwrap() {
    let ts = body(
        "impl Held { pub fn f(&self) -> u32 { *self.cell.lock().unwrap() } }",
        "f",
    );
    assert!(
        !ts.contains("unwrap"),
        "the port's lock() hands back the guard:\n{}",
        ts
    );
}

// ── impl Drop ─────────────────────────────────────────────────────────

#[test]
fn impl_drop_becomes_a_protected_on_drop() {
    let mut fixture = Fixture::build(&[(
        "lib.rs",
        "pub struct Res { pub n: u32 }\n\
         impl Drop for Res { fn drop(&mut self) { let _ = self.n; } }",
    )]);
    let module = fixture.module("lib.rs");
    let Fixture { reg, sink, files } = &mut fixture;
    crate::translate_module(&mut files[0].file, reg, module, sink);
    let ts = crate::codegen::generate_ts(reg, &files[0].file, "lib.rs");
    assert!(
        ts.contains("protected override onDrop(): void {"),
        "`impl Drop` is the type's own cleanup, which AkObject.drop() calls:\n{}",
        ts
    );
    assert!(
        !ts.contains("  drop("),
        "and never an override of drop() itself:\n{}",
        ts
    );
    assert!(ts.contains("extends Drop"), "{}", ts);
}

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
         return Result.Ok(n + 1);",
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
         return Result.Ok(_r0.unwrap() + 1);",
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

#[test]
fn a_question_mark_across_two_error_types_is_reported() {
    let mut fixture = Fixture::build(&[(
        "lib.rs",
        "pub struct Wire;\n\
         pub struct Wrapped;\n\
         pub fn g() -> Result<u32, Wire> { Ok(1) }\n\
         pub fn f() -> Result<u32, Wrapped> { let n = g()?; Ok(n) }",
    )]);
    let _ = fixture.translated_method("lib.rs", "f");
    assert!(
        fixture
            .messages()
            .iter()
            .any(|m| m.contains("through `From`, which the engine has not resolved")),
        "{:?}",
        fixture.messages()
    );
}

// ── Function parameters ───────────────────────────────────────────────

#[test]
fn a_by_value_parameter_is_released_when_the_function_returns() {
    let ts = body("pub fn f(o: Owned) -> u32 { o.n }", "f");
    assert_eq!(
        ts.trim(),
        "try {\n  return o.n;\n} finally {\n  o.drop();\n}",
        "{}",
        ts
    );
}

#[test]
fn a_borrowed_parameter_is_not_released() {
    let ts = body("pub fn f(o: &Owned) -> u32 { o.n }", "f");
    assert!(!ts.contains("drop"), "{}", ts);
}

#[test]
fn a_parameter_the_body_hands_on_is_not_released() {
    let ts = body("pub fn f(o: Owned) -> u32 { take(o) }", "f");
    assert!(!ts.contains("o.drop()"), "{}", ts);
}

// ── Partial moves ─────────────────────────────────────────────────────

#[test]
fn taking_a_droppable_apart_in_a_let_is_reported() {
    let mut fixture = Fixture::build(&[(
        "lib.rs",
        &format!(
            "{}pub fn pair() -> (Owned, Owned) {{ (Owned {{ n: 1 }}, Owned {{ n: 2 }}) }}\n\
             pub fn f() -> u32 {{ let (a, b) = pair(); look(&a) + look(&b) }}",
            PRELUDE
        ),
    )]);
    let _ = fixture.translated_method("lib.rs", "f");
    assert!(
        fixture
            .messages()
            .iter()
            .any(|m| m.contains("takes a droppable value apart")),
        "{:?}",
        fixture.messages()
    );
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

// ── Consuming matches and partial moves ───────────────────────────────

#[test]
fn a_match_that_binds_an_owned_payload_takes_it() {
    let ts = body(
        "pub fn f(c: Choice) -> u32 { match c { Choice::One(v) => v.n, Choice::Two(v) => v.n } }",
        "f",
    );
    assert!(ts.contains("c.intoMatch({"), "the payload is handed over:\n{}", ts);
    assert!(!ts.contains("c.drop()"), "and the enum is moved, not dropped:\n{}", ts);
    assert_eq!(
        ts.matches("v.drop();").count(),
        2,
        "each arm releases what it was given:\n{}",
        ts
    );
}

#[test]
fn a_match_on_a_reference_lends_the_payload() {
    let ts = body(
        "pub fn f(c: &Choice) -> u32 { match c { Choice::One(v) => v.n, Choice::Two(v) => v.n } }",
        "f",
    );
    assert!(ts.contains("c.match({"), "the enum is read, not taken:\n{}", ts);
    assert!(!ts.contains("drop()"), "and nothing is released:\n{}", ts);
}

#[test]
fn a_field_moved_out_of_a_struct_is_taken_from_it() {
    let ts = body("pub fn f(p: Pair) -> u32 { let one = p.one; one.n }", "f");
    assert!(
        ts.contains("p.takeField('one')"),
        "the field leaves the struct:\n{}",
        ts
    );
    assert!(ts.contains("one.drop();"), "the new owner releases it:\n{}", ts);
    assert!(ts.contains("p.drop();"), "and the rest of the struct still drops:\n{}", ts);
}

#[test]
fn a_field_read_through_a_borrow_is_not_a_move() {
    let ts = body("pub fn f(p: &Pair) -> u32 { look(&p.one) }", "f");
    assert!(!ts.contains("takeField"), "a borrow takes nothing:\n{}", ts);
}

#[test]
fn a_result_match_branches_on_is_ok_and_consumes_one_wrapper() {
    let ts = body(
        "pub fn f(n: u32) -> u32 { match fallible(n) { Ok(v) => v.n, Err(_e) => 0 } }",
        "f",
    );
    assert!(ts.contains(".isOk()"), "the test is the Result's own:\n{}", ts);
    assert!(ts.contains(".unwrap();"), "the Ok arm takes the value:\n{}", ts);
    assert!(ts.contains(".unwrapErr();"), "the Err arm takes the error:\n{}", ts);
    assert!(ts.contains("} else {"), "and both arms are emitted:\n{}", ts);
}

// ── Loops ─────────────────────────────────────────────────────────────

#[test]
fn a_loop_over_an_owned_sequence_releases_each_element_and_the_rest() {
    let ts = body(
        "pub fn f(values: Vec<Owned>) -> u32 { let mut t = 0; for v in values { t += look(&v); } t }",
        "f",
    );
    assert!(ts.contains("v.drop();"), "each turn releases its element:\n{}", ts);
    assert!(
        ts.contains(".slice(_at"),
        "and what the loop never reached is released too:\n{}",
        ts
    );
}

#[test]
fn a_loop_over_a_borrowed_sequence_owns_nothing() {
    let ts = body(
        "pub fn f(values: &Vec<Owned>) -> u32 { let mut t = 0; for v in values { t += look(v); } t }",
        "f",
    );
    assert!(ts.contains("for (const v of values)"), "{}", ts);
    assert!(!ts.contains("drop()"), "a borrowing loop releases nothing:\n{}", ts);
}

// ── Assignment ────────────────────────────────────────────────────────

#[test]
fn an_assignment_releases_what_the_place_held() {
    let ts = body(
        "pub fn f() -> u32 { let mut v = Owned { n: 1 }; v = Owned { n: 2 }; v.n }",
        "f",
    );
    let new_value = ts.find("const _a0 = new Owned(2);").expect("the new value first");
    let release = ts.find("v.drop();").expect("then the old one is released");
    let store = ts.find("v = _a0;").expect("then the store");
    assert!(new_value < release && release < store, "in Rust's order:\n{}", ts);
}

// ── Every release through the glue engine ─────────────────────────────

#[test]
fn a_discarded_value_is_released_at_the_end_of_its_statement() {
    let ts = body("pub fn f() { fallible(1); }", "f");
    assert_eq!(ts.trim(), "fallible(1).drop();", "{}", ts);
}

#[test]
fn dropping_a_plain_javascript_value_goes_through_the_cascade() {
    let ts = body("pub fn f(values: Vec<Owned>) -> u32 { drop(values); 0 }", "f");
    assert!(
        ts.contains("dropOwned(values)"),
        "a `Vec` is an array and has no `drop()` of its own:\n{}",
        ts
    );
}

// ── Option payloads ───────────────────────────────────────────────────

#[test]
fn an_unwrap_or_default_is_released_when_it_is_not_chosen() {
    let ts = body(
        "pub fn f(o: Option<Owned>) -> u32 { let d = Owned { n: 9 }; look(&o.unwrap_or(d)) }",
        "f",
    );
    assert!(ts.contains("?? _d0"), "the default is still the fallback:\n{}", ts);
    assert!(
        ts.contains("!== _d0") && ts.contains("_d0.drop();"),
        "and it is released on the path that did not take it:\n{}",
        ts
    );
}

// ── Closures ──────────────────────────────────────────────────────────

#[test]
fn an_immediately_invoked_move_closure_releases_its_captures_inside_itself() {
    let ts = body(
        "pub fn f() -> u32 { let a = Owned { n: 1 }; (move || look(&a))() }",
        "f",
    );
    assert!(!ts.contains("OwnedClosure"), "nothing outlives the call:\n{}", ts);
    assert!(ts.contains("} finally {\n    a.drop();"), "{}", ts);
}

#[test]
fn a_move_closure_bound_to_a_local_is_called_through_the_runtime() {
    let ts = body(
        "pub fn f() -> u32 { let a = Owned { n: 1 }; let g = move || look(&a); g() }",
        "f",
    );
    assert!(ts.contains("new OwnedClosure([a],"), "{}", ts);
    assert!(ts.contains("g.call()"), "a closure that owns is never a bare call:\n{}", ts);
    assert!(ts.contains("g.drop();"), "and the block releases it:\n{}", ts);
}

#[test]
fn a_closure_with_nothing_droppable_to_capture_stays_a_plain_function() {
    let ts = body("pub fn f() -> u32 { let n = 1u32; (move || n)() }", "f");
    assert!(!ts.contains("OwnedClosure"), "{}", ts);
}

// ── Macros ────────────────────────────────────────────────────────────

#[test]
fn a_value_moved_into_a_vec_macro_is_not_released_by_the_block() {
    let ts = body(
        "pub struct Bag { pub items: Vec<Owned> }\n\
         pub fn f() -> Bag { let a = Owned { n: 1 }; Bag { items: vec![a] } }",
        "f",
    );
    assert!(ts.contains("new Bag([a])"), "{}", ts);
    assert!(!ts.contains("a.drop()"), "the bag owns it now:\n{}", ts);
}

#[test]
fn a_format_argument_borrows_rather_than_moves() {
    let ts = body(
        "pub fn f(p: &Pair) -> String { format!(\"{}\", p.one.n) }",
        "f",
    );
    assert!(!ts.contains("takeField"), "`Display` takes `&self`:\n{}", ts);
}

#[test]
fn an_unexpanded_macro_handed_an_owned_value_is_reported() {
    let mut fixture = Fixture::build(&[(
        "lib.rs",
        &format!(
            "{}pub fn f() -> u32 {{ let a = Owned {{ n: 1 }}; nowhere!(a); 0 }}",
            PRELUDE
        ),
    )]);
    let _ = fixture.translated_method("lib.rs", "f");
    assert!(
        fixture
            .messages()
            .iter()
            .any(|m| m.contains("`nowhere!` is emitted as a comment and is handed `a`")),
        "{:?}",
        fixture.messages()
    );
}

// ── Tail and return positions ─────────────────────────────────────────

#[test]
fn a_branch_that_hands_a_local_away_in_tail_position_sets_its_flag() {
    let ts = body(
        "pub fn f(c: bool) -> u32 { let a = Owned { n: 1 }; if c { take(a) } else { look(&a) } }",
        "f",
    );
    assert!(ts.contains("_moved0 = true;"), "the flag is set in the branch:\n{}", ts);
    assert!(
        ts.contains("if (!_moved0) a.drop();"),
        "and the `finally` reads it:\n{}",
        ts
    );
}

// ── Self ──────────────────────────────────────────────────────────────

#[test]
fn a_method_that_takes_self_by_value_owns_the_receiver() {
    let ts = body(
        "impl Owned { pub fn into_n(self) -> u32 { self.n } }",
        "into_n",
    );
    assert_eq!(
        ts.trim(),
        "try {\n  return this.n;\n} finally {\n  this.drop();\n}",
        "{}",
        ts
    );
}

#[test]
fn a_borrowing_method_does_not_release_its_receiver() {
    let ts = body("impl Owned { pub fn get(&self) -> u32 { self.n } }", "get");
    assert!(!ts.contains("this.drop()"), "{}", ts);
}

// ── What a type says it owns ──────────────────────────────────────────

#[test]
fn owned_fields_is_read_off_the_whole_field_type() {
    let mut fixture = Fixture::build(&[(
        "lib.rs",
        &format!(
            "{}pub struct Holder {{ pub borrowed: Vec<&'static Owned>, pub own: Vec<Owned> }}",
            PRELUDE
        ),
    )]);
    let _ = fixture.translated_method("lib.rs", "take");
    let ts = crate::codegen::generate_ts(&fixture.reg, &fixture.files[0].file, "lib.rs");
    assert!(
        ts.contains("return [this.own];"),
        "a `Vec<&T>` is an array of borrows and owns nothing:\n{}",
        ts
    );
}

// ── Where a method comes from ─────────────────────────────────────────

#[test]
fn the_trait_in_scope_filter_runs_over_the_whole_deref_chain() {
    let mut fixture = Fixture::build(&[(
        "lib.rs",
        "use std::sync::RwLock;\n\
         use std::cell::RefCell;\n\
         pub struct Cells { pub cells: RwLock<RefCell<Vec<u32>>> }\n\
         impl Cells {\n\
           pub fn push(&self, v: u32) { let g = self.cells.read().unwrap(); g.borrow_mut().push(v); }\n\
           pub fn count(&self) -> usize { let i = self.cells.read().unwrap(); i.borrow().len() }\n\
         }",
    )]);
    let push = fixture.translated_method("lib.rs", "push");
    assert!(
        push.contains("g.value.borrowMut()"),
        "the reflexive `BorrowMut` blanket must not win at depth 0:\n{}",
        push
    );
    let count = fixture.translated_method("lib.rs", "count");
    assert!(count.contains("_t0.value.length"), "{}", count);
}

// ── Untyped locals ────────────────────────────────────────────────────

#[test]
fn a_local_the_engine_cannot_type_says_that_nothing_releases_it() {
    let mut fixture = Fixture::build(&[(
        "lib.rs",
        &format!(
            "{}pub fn f(o: &Owned) -> u32 {{ let x = o.mystery(); 0 }}",
            PRELUDE
        ),
    )]);
    let _ = fixture.translated_method("lib.rs", "f");
    assert!(
        fixture
            .messages()
            .iter()
            .any(|m| m.contains("local `x` is left untyped, so nothing releases whatever it holds")),
        "{:?}",
        fixture.messages()
    );
}

// ── tokio ─────────────────────────────────────────────────────────────

#[test]
fn tokios_mutex_is_the_async_one_and_its_guard_is_released() {
    let mut fixture = Fixture::build(&[(
        "lib.rs",
        "use tokio::sync::Mutex;\n\
         pub struct Cell { pub inner: Mutex<u32> }\n\
         impl Cell { pub async fn read(&self) -> u32 { let g = self.inner.lock().await; *g } }",
    )]);
    let ts = fixture.translated_method("lib.rs", "read");
    assert_eq!(
        ts.trim(),
        "const g = await this.inner.lock();\n\
         try {\n  \
           return g.value;\n\
         } finally {\n  \
           g.drop();\n\
         }",
        "{}",
        ts
    );
    let emitted = crate::codegen::generate_ts(&fixture.reg, &fixture.files[0].file, "lib.rs");
    assert!(
        emitted.contains("AsyncMutex<number>"),
        "tokio's Mutex is not std's:\n{}",
        emitted
    );
}

#[test]
fn a_select_races_tagged_branches_and_releases_all_of_them() {
    let mut fixture = Fixture::build(&[(
        "lib.rs",
        "use tokio::sync::{mpsc, Notify};\n\
         pub async fn wait(n: &Notify, mut rx: mpsc::Receiver<()>) {\n  \
           let ready = n.notified();\n  \
           tokio::select! {\n    \
             _ = ready => {}\n    \
             _ = rx.recv() => {}\n  \
           }\n\
         }",
    )]);
    let ts = fixture.translated_method("lib.rs", "wait");
    assert!(ts.contains("await select("), "{}", ts);
    assert!(ts.contains("{ tag: '_0', promise: ready }"), "{}", ts);
    assert!(
        ts.contains("dropOwned(_v2.promise)") || ts.contains(".promise);"),
        "every branch is released when the select returns:\n{}",
        ts
    );
    assert!(
        !ts.contains("ready.drop()"),
        "and the block that named a branch does not release it as well:\n{}",
        ts
    );
}

// ── Bindings a pattern made ───────────────────────────────────────────

#[test]
fn a_while_let_binding_is_released_each_turn() {
    let mut fixture = Fixture::build(&[(
        "lib.rs",
        &format!(
            "{}pub struct Queue {{ pub items: Vec<Owned> }}\n\
             pub fn f(q: &mut Queue) -> u32 {{ let mut t = 0; while let Some(v) = q.items.pop() {{ t += look(&v); }} t }}",
            PRELUDE
        ),
    )]);
    let ts = fixture.translated_method("lib.rs", "f");
    assert!(ts.contains("for (;;) {"), "{}", ts);
    let loop_at = ts.find("for (;;)").expect("the loop");
    let release = ts.find("v.drop();").expect("the binding is released");
    assert!(release > loop_at, "inside the loop, not after it:\n{}", ts);
}
