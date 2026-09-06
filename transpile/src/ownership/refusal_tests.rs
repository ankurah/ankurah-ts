//! What a statement that REFUSED owes the values it was going to hand away.
//!
//! For: an R12 hole throws where the lowering had no answer, so every move the
//! statement was going to perform does not happen. The ownership emission had
//! already decided otherwise — it decides from the SOURCE, before any refusal
//! is known — so the move flag was set and the block's `finally` left the value
//! alone. K3.

use crate::testing::Fixture;

/// K3: a statement that REFUSED hands nothing away, so it sets no move flag.
///
/// `core/value/cast_predicate.ts`'s `ExprList` arm wrote `_moved3 = true` and
/// then refused the `collect` on the next line, so the arm's `finally` read a
/// flag that was true and released nothing: the payload the arm was handed
/// leaked on every call.
#[test]
fn a_refused_statement_sets_no_move_flag() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         pub fn tally<T>(x: T) -> u32 { 0 }\n\
         pub fn refused(tokens: Vec<Token>, flag: bool) -> u32 {\n\
           let held = tokens;\n\
           if flag { return 0; }\n\
           let built = held.into_iter().map(|t| t).collect();\n\
           tally(built)\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "refused");
    assert!(ts.contains("unsupported("), "the collect was expected to refuse: {}", ts);
    let flag_set = ts.lines().position(|l| l.trim().starts_with("_moved") && l.contains("= true;"));
    assert!(flag_set.is_none(), "a refused statement set a move flag:\n{}", ts);
}

/// The other half: a local the analysis marked handed away on EVERY path has
/// no flag and no `finally` to fall back on, so the refusal releases it where
/// the hand-away would have been.
///
/// `core/property/backend/lww.ts`'s `from_state_buffer` is the shape — the map
/// its decoder had just built was moved into a `collect` the engine has no
/// construction for. (Its map is not droppable in the engine's answer today,
/// so no corpus site takes this path; this is the rule, tested.)
#[test]
fn a_refused_statement_releases_what_it_was_going_to_hand_away() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         pub fn tally<T>(x: T) -> u32 { 0 }\n\
         pub fn refused(tokens: Vec<Token>) -> u32 {\n\
           let held = tokens;\n\
           let built = held.into_iter().map(|t| t).collect();\n\
           tally(built)\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "refused");
    let released = ts.find("dropOwned(held);").expect(&format!("nothing released `held`:\n{}", ts));
    let hole = ts.find("unsupported(").expect("the collect was expected to refuse");
    assert!(released < hole, "the release stands below the hole that throws:\n{}", ts);
}
