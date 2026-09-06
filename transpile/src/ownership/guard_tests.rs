//! Guards on the runtime's own match forms.
//!
//! For: `.match({..})` and `.intoMatch({..})` dispatch on the variant NAME, so
//! a guard cannot be part of the key and an arm whose guard fails has to reach
//! the arm below it. Each of these writes a guarded match of one shape and
//! reads back which arm ran, where the guard's own temporaries were released,
//! and what the arm below was left holding.

use crate::testing::Fixture;

use super::tests::{body, PRELUDE};


#[test]
fn a_guarded_option_match_is_tried_arm_by_arm() {
    let ts = body(
        "pub fn f(o: Option<u32>) -> u32 { match o { Some(v) if v > 2 => v, Some(_) => 1, None => 0 } }",
        "f",
    );
    // PREMISE CHANGED 2026-09-05 (step 9a slice 2, H12): the labelled block is
    // taken only where a branch has to jump over what stands after it. Every
    // arm here returns, so there is nothing to jump over and no label.
    assert!(!ts.contains("break _match"), "no arm jumps, so none is written:\n{}", ts);
    let guarded = ts.find("if (v > 2)").expect("the guard is tested");
    let below = ts.find("return 1;").expect("the arm below it");
    assert!(guarded < below, "and a guard that failed falls through:\n{}", ts);
}

#[test]
fn a_guarded_borrowing_enum_match_is_tried_arm_by_arm() {
    let ts = body(
        "pub fn f(c: &Choice) -> u32 { \
           match c { Choice::One(o) if o.n > 2 => 9, Choice::One(o) => look(o), Choice::Two(o) => look(o) } }",
        "f",
    );
    // A match that only READS its subject keeps the if-chain, which has carried
    // guards since before the arm chain existed: the chain is for a CONSUMING
    // match, where nothing else can mark the subject moved. Both arms naming
    // `One` are written, which is what this test is for.
    assert!(
        ts.matches("c.is('One')").count() == 2,
        "both arms naming `One` are written, which one key in a `.match({{}})` cannot \
         carry:\n{}",
        ts
    );
    let guard = ts.find("if (o.n > 2)").expect(&ts);
    let below = ts.find("look(o)").expect(&ts);
    assert!(
        !inside_the_branch(&ts, guard, below),
        "the arm below stands inside the guard's own branch, so a failed guard never \
         reaches it:\n{}",
        ts
    );
}

#[test]
fn a_guarded_consuming_enum_arm_is_written_by_the_chain() {
    // PREMISE CHANGED 2026-09-05 (fixpass6 item 2, D8): a guarded consuming
    // match used to report "a failed guard cannot fall out of" and drop the
    // guard, so the arm ran for every value its pattern matched — live at
    // `core/src/node.rs:621`, where an EMPTY event bridge answered the bridge
    // path. The chain binds the names inside the arrow, tests the guard there,
    // and falls through to the arm below.
    let mut fixture = Fixture::build(&[(
        "lib.rs",
        &format!(
            "{}pub fn f(c: Choice) -> u32 {{ \
               match c {{ Choice::One(o) if o.n > 2 => take(o), Choice::One(o) => take(o), \
                          Choice::Two(o) => take(o) }} }}\n",
            PRELUDE
        ),
    )]);
    let ts = fixture.translated_method("lib.rs", "f");
    assert!(ts.contains("intoMatch"), "the subject is still handed over:\n{}", ts);
    assert!(!ts.contains("unsupported("), "{}", ts);
    // Amended (step 9a slice 2, G2): the guard is made in a `try`, so its test
    // is held in a name.
    // The guarded body is inside the branch the guard's `if` opens, and the arm
    // below it is outside — which is what "a failed guard falls through" means.
    let branch = ts.find("if (_g0)").expect(&ts);
    let guarded = ts.find("take(o)").expect(&ts);
    let below = ts.rfind("take(o)").expect(&ts);
    assert!(inside_the_branch(&ts, branch, guarded), "the guarded arm is not guarded:\n{}", ts);
    assert!(
        !inside_the_branch(&ts, branch, below),
        "the arm below stands inside the guard's branch:\n{}",
        ts
    );
    let said = fixture.messages();
    assert!(
        said.iter().all(|m| !m.contains("guard is dropped")),
        "and nothing says the guard was dropped: {:?}",
        said
    );
}

#[test]
fn a_guarded_result_arm_is_written_and_falls_through_to_the_arm_below() {
    // PREMISE CHANGED 2026-09-05 (fixpass6 item 2, D8): a guarded `Result` arm
    // used to be reported and dropped, so the arm ran unconditionally — live at
    // `core/src/context.rs:187`, where `Err(NoDurablePeers) if cached => ()`
    // vanished entirely and a cached entity with no durable peers answered an
    // error. The side reads its payload ONCE and tries its arms against it.
    // Amended (step 9a slice 2, G2): the guard is made in a `try` of its own, so
    // the test is held in a name and the `if` reads the name.
    let mut fixture = Fixture::build(&[(
        "lib.rs",
        &format!(
            "{}pub fn f(r: Result<Owned, Oops>) -> u32 {{ \
               match r {{ Ok(v) if v.n > 2 => take(v), Ok(v) => take(v), Err(_) => 0 }} }}\n",
            PRELUDE
        ),
    )]);
    let ts = fixture.translated_method("lib.rs", "f");
    assert_eq!(ts.matches(".unwrap()").count(), 1, "the payload is read once:\n{}", ts);
    let branch = ts.find("if (_g0)").expect(&ts);
    let guarded = ts.find("take(v)").expect(&ts);
    let below = ts.rfind("take(v)").expect(&ts);
    assert!(inside_the_branch(&ts, branch, guarded), "the guarded arm is not guarded:\n{}", ts);
    assert!(
        !inside_the_branch(&ts, branch, below),
        "the arm below stands inside the guard's branch:\n{}",
        ts
    );
    assert!(
        fixture.messages().iter().all(|m| !m.contains("takes the wrapper apart")),
        "and nothing says the guard was dropped: {:?}",
        fixture.messages()
    );
}

/// Does `at` stand INSIDE the block the `if` at `opens` starts?
///
/// K14: three tests asserted `guard < below` — that the guard's test is written
/// before the arm below it — which is true of a chain that works and equally
/// true of one whose fall-through is broken, because both arms are written
/// either way. What has to hold is the STRUCTURE: the guarded body stands
/// inside the branch the guard's `if` opens, and the arm below it stands after
/// that branch has closed.
fn inside_the_branch(ts: &str, opens: usize, at: usize) -> bool {
    let mut depth = 0i32;
    let mut entered = false;
    for (offset, c) in ts[opens..].char_indices() {
        if opens + offset >= at {
            return entered && depth > 0;
        }
        match c {
            '{' => {
                depth += 1;
                entered = true;
            }
            '}' => depth -= 1,
            _ => {}
        }
    }
    false
}
