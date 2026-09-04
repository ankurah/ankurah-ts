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
pub fn take(o: Owned) -> u32 { o.n }\n\
pub fn look(o: &Owned) -> u32 { o.n }\n\
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
fn a_local_moved_into_a_closure_is_not_released_and_is_reported() {
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
        fixture
            .messages()
            .iter()
            .any(|m| m.contains("the closure takes ownership of `a`")),
        "the capture is reported: {:?}",
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
