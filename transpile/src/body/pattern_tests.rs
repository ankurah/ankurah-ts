//! What the pattern machinery writes: the tests a pattern asks and the names it
//! takes out of the value.

use crate::control_flow::sentinel_tests::inside_an_arrow;
use crate::testing::Fixture;

fn built(src: &str) -> Fixture {
    Fixture::build(&[("lib.rs", src)])
}

/// Rust's `_` takes no name. TypeScript's `_` is a variable called `_`, so two
/// of them in one block are a duplicate declaration and a JavaScript engine
/// refuses the module. `core/src/resultset.ts` was one.
#[test]
fn two_ignored_tuple_members_declare_nothing() {
    let mut f = built(
        "pub fn pick(a: Option<u32>, b: Option<u32>) -> u32 {\n\
           match (&a, &b) {\n\
             (Some(_), None) => 1,\n\
             (None, Some(_)) => 2,\n\
             _ => 3,\n\
           }\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "pick");
    assert!(!ts.contains("const _ ="), "{}", ts);
    assert!(ts.contains("(_v[0] != null) && (_v[1] == null)"), "{}", ts);
    assert!(ts.contains("(_v[0] == null) && (_v[1] != null)"), "{}", ts);
}

/// A `_` field of a struct pattern is not written into the destructuring at
/// all: `const { left, operator: _, right: _ }` was two `_` keys in one
/// declaration (`storage-common/planner.ts`).
#[test]
fn an_ignored_struct_field_is_left_out_of_the_destructuring() {
    let mut f = built(
        "pub enum Predicate { Comparison { left: u32, operator: u32, right: u32 }, True }\n\
         pub fn left_of(p: &Predicate) -> u32 {\n\
           match p { Predicate::Comparison { left, operator: _, right: _ } => *left, _ => 0 }\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "left_of");
    assert!(ts.contains("const left = v.left;"), "{}", ts);
    assert!(!ts.contains("operator"), "{}", ts);
    assert!(!ts.contains("right"), "{}", ts);
}

/// `let (field, _, _) = triple;` destructures with holes, which is how
/// JavaScript says "skip this one".
#[test]
fn an_ignored_tuple_element_of_a_let_is_a_hole() {
    let mut f = built(
        "pub fn first(t: (u32, u32, u32)) -> u32 { let (field, _, _) = t; field }",
    );
    let ts = f.translated_method("lib.rs", "first");
    assert!(ts.contains("const [field, , ] = t;"), "{}", ts);
}

/// A closure that ignores two arguments cannot call both of them `_`.
/// `core/src/property/backend/yjs.ts` emitted `(_, _) => ..`, which no
/// JavaScript engine will parse.
#[test]
fn two_ignored_closure_parameters_get_distinct_names() {
    let mut f = built(
        "pub fn run<F: Fn(u32, u32)>(f: F) { f(1, 2) }\n\
         pub fn go() { run(|_, _| { }) }",
    );
    let ts = f.translated_method("lib.rs", "go");
    assert!(ts.contains("(_, __) =>"), "{}", ts);
}

/// `Some(_)` asks whether the value is there and takes nothing out of it.
#[test]
fn an_ignored_option_payload_is_a_test_and_no_binding() {
    let mut f = built(
        "pub fn present(a: &Option<u32>) -> u32 { match a { Some(_) => 1, None => 0 } }",
    );
    let ts = f.translated_method("lib.rs", "present");
    assert!(!ts.contains("const _"), "{}", ts);
    assert!(ts.contains("!= null"), "{}", ts);
}

/// A `?` written inside a body the emitter lifts into an arrow must leave the
/// FUNCTION. Before the sentinel, `Result.Err(..)` came back as the value of
/// the `if` — and `Result.Err` is a truthy object, so `if (applied)` took the
/// success branch for a call that had failed. Ten sites, the worst of them
/// `commit_remote_transaction`, which wrote state for an event it could not
/// apply.
#[test]
fn a_question_mark_in_a_lifted_body_leaves_the_function() {
    let mut f = built(
        "pub enum E { Refused }\n\
         pub struct S;\n\
         impl S {\n\
           pub fn apply(&self, ok: bool) -> Result<bool, E> { Ok(ok) }\n\
           pub fn commit(&self, already: bool, ok: bool) -> Result<u32, E> {\n\
             let applied = if already { true } else { self.apply(ok)? };\n\
             if applied { Ok(1) } else { Ok(0) }\n\
           }\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "commit");
    assert!(
        inside_an_arrow(&ts, "return { $jump: 'return', $value: Result.Err("),
        "the `?` inside the arrow has to hand the exit back, not return from the arrow:\n{}",
        ts
    );
    assert!(
        !inside_an_arrow(&ts, "?.$jump === 'return') return"),
        "and the statement that performs it stands outside the arrow:\n{}",
        ts
    );
}

/// The same for a plain `return`, and for a block used as a value.
#[test]
fn a_return_in_a_lifted_block_leaves_the_function() {
    let mut f = built(
        "pub fn pick(stop: bool) -> u32 {\n\
           let n = { if stop { return 7; } 3 };\n\
           n + 1\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "pick");
    assert!(
        inside_an_arrow(&ts, "return { $jump: 'return', $value: 7 }"),
        "the `return` is written inside the block's arrow:\n{}",
        ts
    );
    assert!(
        !inside_an_arrow(&ts, "?.$jump === 'return') return"),
        "and the test that performs it stands outside it:\n{}",
        ts
    );
}

/// An arm of a consuming match written as a STATEMENT is an arrow function too,
/// and its `?` used to return from the arrow while the match's value was thrown
/// away. In return position the arm's `return` really is the function's, and
/// that shape is left alone.
#[test]
fn an_arm_of_a_statement_match_hands_its_exit_back() {
    let mut f = built(
        "pub enum E { Refused }\n\
         pub enum Step { Skip, Apply(bool) }\n\
         pub fn apply(ok: bool) -> Result<bool, E> { Ok(ok) }\n\
         pub fn run(step: Step) -> Result<u32, E> {\n\
           let mut n = 0u32;\n\
           match step {\n\
             Step::Skip => {}\n\
             Step::Apply(ok) => { if apply(ok)? { n += 1; } }\n\
           }\n\
           Ok(n)\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "run");
    assert!(
        inside_an_arrow(&ts, "return { $jump: 'return', $value: Result.Err("),
        "the arm hands its exit back from inside its arrow:\n{}",
        ts
    );
    assert!(
        !inside_an_arrow(&ts, "?.$jump === 'return') return"),
        "and the statement after the match performs it:\n{}",
        ts
    );
}

/// A `?` inside a closure belongs to the closure and is not the enclosing
/// function's exit, so a lifted body containing only that one is written as it
/// always was.
#[test]
fn a_question_mark_inside_a_closure_is_not_the_functions_exit() {
    use crate::control_flow::sentinel::leaves_the_function;
    let expr: syn::Expr = syn::parse_str("{ let f = |x: u32| -> Result<u32, ()> { g(x)? }; 1 }")
        .expect("parses");
    assert!(!leaves_the_function(&expr));
    let expr: syn::Expr = syn::parse_str("{ let y = g()?; y }").expect("parses");
    assert!(leaves_the_function(&expr));
}

/// A module-level `const` carries its VALUE. `ConstInfo` used to hold the
/// const's type and nothing else, so `human_id`'s word list — the thing
/// `humanize` indexes — came out `undefined as any`.
#[test]
fn a_const_carries_its_initialiser() {
    let mut f = built("pub const TAG: u8 = 0x04;\npub fn tag() -> u8 { TAG }");
    let ts = f.emitted("lib.rs");
    assert!(ts.contains("export const TAG: number = 4;"), "{}", ts);
    assert!(!ts.contains("undefined as any"), "{}", ts);
}

/// A `static` had no arm in the item walk at all, so the item vanished and
/// every use of it named nothing.
#[test]
fn a_static_is_an_item() {
    let mut f = built("pub static NAME: &str = \"sys\";\npub fn name() -> String { NAME.to_string() }");
    let ts = f.emitted("lib.rs");
    assert!(ts.contains("export const NAME: string = 'sys';"), "{}", ts);
}

/// A struct literal is emitted in the order the CONSTRUCTOR takes its
/// parameters, which is the order the fields are declared — not the order the
/// literal happened to write them.
#[test]
fn a_struct_literal_is_written_by_field_name() {
    let mut f = built(
        "pub struct Rec { pub first: u32, pub second: u32, pub third: bool }\n\
         impl Rec { pub fn make(a: u32, b: u32, c: bool) -> Rec { Self { third: c, first: a, second: b } } }",
    );
    let ts = f.translated_method("lib.rs", "make");
    assert!(ts.contains("new Rec(a, b, c)"), "{}", ts);
}

/// `..base` fills every field the literal does not name, and nothing here
/// reads it, so the site says so rather than leaving those fields undefined in
/// silence.
#[test]
fn a_functional_update_base_is_reported() {
    let f = built(
        "pub struct Pair { pub a: u32, pub b: u32 }\n\
         impl Pair { pub fn tweak(base: Pair, a: u32) -> Pair { Pair { a, ..base } } }",
    );
    let mut f = f;
    let _ = f.translated_method("lib.rs", "tweak");
    assert!(
        f.messages().iter().any(|m| m.contains("`..` fills the fields")),
        "{:?}",
        f.messages()
    );
}

/// `let PAT = e else { .. }` tests the pattern and runs the else. Both were
/// dropped: `let ScanState::Scanning { .. } = state else { return None };` came
/// out as the destructuring alone, so the variant was never tested and the
/// `return None` was gone. Twelve sites.
#[test]
fn a_let_else_tests_its_pattern_and_runs_the_else() {
    let mut f = built(
        "pub enum ScanState { Idle, Scanning { stream: u32, len: u32 } }\n\
         pub fn read(state: &ScanState) -> Option<u32> {\n\
           let ScanState::Scanning { stream, len } = state else { return None };\n\
           Some(stream + len)\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "read");
    assert!(ts.contains("is('Scanning')"), "the variant is tested:\n{}", ts);
    assert!(ts.contains("return null;"), "the else branch runs:\n{}", ts);
    assert!(!ts.contains("/* let-else */"), "{}", ts);
}

/// A `let … else` that shadows renames the DECLARATION as well as the uses.
/// `const [queryId, ..] = _v;` beside a parameter of that name is a duplicate
/// declaration, and bun refuses the module.
#[test]
fn a_let_else_that_shadows_renames_what_it_declares() {
    let mut f = built(
        "pub fn read(k: u32, m: Option<u32>) -> u32 {\n\
           let k = k + 1;\n\
           let Some(k) = m else { return 0 };\n\
           k + 5\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "read");
    assert!(!ts.contains("const k = k"), "{}", ts);
    assert!(ts.contains("const k_1 = checkedAdd(k, 1,"), "{}", ts);
    assert!(ts.contains("const k_2 ="), "{}", ts);
    assert!(ts.contains("checkedAdd(k_2, 5,"), "{}", ts);
}

/// A pattern may bind the SUBJECT's own name — `match b { Some(b) => b + 1 }`
/// is ordinary Rust, and a shadow there is what the source meant. `const b = b`
/// is `ReferenceError: Cannot access 'b' before initialization`.
#[test]
fn a_pattern_that_binds_its_subjects_name_reads_it_into_a_temporary() {
    let mut f = built("pub fn read(b: Option<u32>) -> u32 { match b { Some(b) => b + 1, None => 0 } }");
    let ts = f.translated_method("lib.rs", "read");
    assert!(!ts.contains("const b = b;"), "{}", ts);
    assert!(ts.contains("const _v = b;"), "{}", ts);
    assert!(ts.contains("const b = _v;"), "{}", ts);
}

/// `Some`/`None` are decided by IDENTITY. A crate enum with a variant of that
/// name is a different value: `enum State { None, Some(i32), Other }` under a
/// guard came out `if (s != null) { const n = s; …` — arm one ran for
/// `State::Other` and `State::None` was dead.
#[test]
fn a_crate_enums_some_and_none_are_not_options() {
    let mut f = built(
        "pub enum State { None, Some(i32), Other }\n\
         pub fn read(s: State, flag: bool) -> i32 {\n\
           match s { State::Some(n) if flag => n, State::None => -1, _ => 0 }\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "read");
    assert!(ts.contains("s.is('Some')"), "{}", ts);
    assert!(ts.contains("s.is('None')"), "{}", ts);
    assert!(!ts.contains("== null"), "{}", ts);
}

/// `n @ 1` binds AND asks. Dropping the subpattern made `Some(n @ 1) => n` an
/// arm that matches every `Some`, so `o == 7` took it.
#[test]
fn a_subpattern_is_a_test_as_well_as_a_binding() {
    let mut f = built("pub fn read(o: Option<u32>) -> u32 { match o { Some(n @ 1) => n, Some(m) => m, None => 0 } }");
    let ts = f.translated_method("lib.rs", "read");
    assert!(ts.contains("o === 1"), "{}", ts);
}

/// A `None` NESTED in a pattern is not a binding either. `is_irrefutable` was
/// given no such exception, so `Some(None) => -1` emitted an arm that runs for
/// `Some(5)`.
#[test]
fn a_nested_none_is_a_test_and_not_a_name() {
    let mut f = built("pub fn read(o: Option<Option<u32>>) -> i32 { match o { Some(None) => -1, Some(Some(v)) => v as i32, None => 0 } }");
    let ts = f.translated_method("lib.rs", "read");
    assert!(!ts.contains("const none ="), "{}", ts);
    assert!(ts.contains("o != null && (o == null)"), "{}", ts);
}

/// C1, the caller's half: a local this body hands out as `&mut` lives in the
/// cell from its `let`, so nothing has to be unboxed after the call — and a
/// cell handed to another `&mut` parameter goes over WHOLE, because Rust
/// reborrows there.
#[test]
fn a_local_handed_out_as_mut_lives_in_a_cell() {
    let mut f = built(
        "fn fill(buffer: &mut String, found: &mut usize) { buffer.push_str(\"?\"); *found += 1; }\n\
         fn again(buffer: &mut String, found: &mut usize) { fill(buffer, found); }\n\
         pub fn render() -> String {\n\
           let mut found = 0;\n\
           let mut buffer = String::new();\n\
           fill(&mut buffer, &mut found);\n\
           buffer\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "render");
    assert!(ts.contains("const found = new BorrowMut(0);"), "{}", ts);
    assert!(ts.contains("const buffer = new BorrowMut('');"), "{}", ts);
    assert!(ts.contains("fill(buffer, found);"), "{}", ts);
    assert!(ts.contains("return buffer.value;"), "{}", ts);
    let ts = f.translated_method("lib.rs", "again");
    assert!(ts.contains("fill(buffer, found);"), "a reborrow hands the cell over:\n{}", ts);
}

/// R3: an operand is a VALUE position. Asked as an ordinary expression, a
/// value-position `if` put an `if` STATEMENT between the brackets and inside a
/// unary operand — output no JavaScript engine will parse, and no diagnostic.
#[test]
fn a_value_position_if_inside_an_operand_is_an_expression() {
    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        "pub fn pick(rows: &Vec<u32>, ok: bool) -> u32 { rows[if ok { 1 } else { 2 }] }\n\
         pub fn negated(ok: bool) -> i32 { -{ if ok { 1i32 } else { 2i32 } } }",
    )]);
    let ts = f.emitted("lib.rs");
    assert!(ts.contains("rows[(ok ? 1 : 2)]"), "{ts}");
    // K8 (2026-09-06): `-` on a resolved SIGNED width goes through the
    // runtime's `checkedNeg`, which raises where Rust raises. What this test is
    // about is unchanged — the operand is still the ternary and not an `if`
    // statement.
    assert!(ts.contains("checkedNeg((ok ? 1 : 2), 'i32')"), "{ts}");
    assert!(!ts.contains("[if ("), "a statement stands between the brackets:\n{ts}");
}

/// R8: the position a call stands in is the CALLER's answer, and the path for a
/// receiver the engine could not resolve used to say "read as a value" whatever
/// the caller said — so a `*entry(k).or_insert(0) += 1` on that path would have
/// been written `.value.value`.
#[test]
fn an_unresolved_call_written_through_is_not_read_as_a_value() {
    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        "use std::collections::HashMap;\n\
         pub struct Counts { pub m: HashMap<String, u32> }\n\
         impl Counts { pub fn bump(&mut self, k: String) { \
         *self.m.entry(k).or_insert(0) += 1; } }",
    )]);
    let ts = f.emitted("lib.rs");
    assert!(!ts.contains(".value.value"), "the slot was read twice:\n{ts}");
}

/// K9: `Variant(..)` matches every value of that variant, and the variant key
/// IS the test.
///
/// Read as refutable, `..` went to `pattern_test`, which has no test to write
/// for it: the arm carried a hole that threw before the body the source wrote
/// could run. Its member is not declared either — `const ... = v._0;` is not a
/// declaration a JavaScript engine will read.
#[test]
fn a_rest_pattern_covers_the_members_no_name_took() {
    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        "pub enum Wide { Two(u32, u32), One(u32) }\n\
         pub fn covered(w: &Wide) -> u32 { match w { Wide::Two(..) => 2, Wide::One(n) => *n } }\n\
         pub fn first_of(w: &Wide) -> u32 { match w { Wide::Two(a, ..) => *a, Wide::One(n) => *n } }",
    )]);
    let ts = f.emitted("lib.rs");
    assert!(!ts.contains("unsupported("), "a `..` refused:\n{ts}");
    assert!(!ts.contains("..."), "a `..` was declared as a name:\n{ts}");
    assert!(ts.contains("Two: (v) => 2,"), "{ts}");
    assert!(ts.contains("const a = v._0;"), "{ts}");
}

/// And a `..` written anywhere but LAST is refused: each element takes the
/// member at its own position, so every name after the `..` would be bound
/// from the wrong member — which never says so on its own.
#[test]
fn a_rest_pattern_before_the_last_element_is_refused() {
    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        "pub enum Three { All(u32, u32, u32), Nothing }\n\
         pub fn last_of(w: &Three) -> u32 { match w { Three::All(.., c) => *c, Three::Nothing => 0 } }",
    )]);
    let ts = f.emitted("lib.rs");
    assert!(ts.contains("unsupported("), "{ts}");
    assert!(
        f.messages().iter().any(|m| m.contains("bound from the wrong member")),
        "{:?}",
        f.messages()
    );
}
