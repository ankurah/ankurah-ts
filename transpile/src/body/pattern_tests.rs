//! What the pattern machinery writes: the tests a pattern asks and the names it
//! takes out of the value.

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
