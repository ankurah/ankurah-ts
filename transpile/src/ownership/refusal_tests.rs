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
    // R9/D11: the release used to stand ABOVE the statement, on the reasoning
    // that nothing of a refused statement has run. That is false wherever part
    // of it HAS run — a `?` operand to the left of the hole, or a hoist above
    // it — so there is now one shape for every refusal: a `finally` around the
    // statement, which a throw leaves through just as surely.
    assert!(
        ts.contains("dropOwned(held);"),
        "nothing released `held`:\n{}",
        ts
    );
    let hole = ts.find("unsupported(").expect("the collect was expected to refuse");
    let released = ts.find("dropOwned(held);").expect("just checked");
    assert!(
        hole < released && ts[..hole].contains("try {"),
        "the release stands in a `finally` the hole throws out through:\n{}",
        ts
    );
}

/// A method declared `self` takes its receiver with it, and a receiver that is
/// a FIELD of a place is a partial move.
///
/// Rust takes the field out of the struct and leaves the rest where it was.
/// Written as a plain property read the struct and the callee both owned it, so
/// the block's own `pair.drop()` released a value the callee had already taken
/// — `BUG: Entity was used after being moved`. Six of ankql's seven
/// `ast.test.ts` failures are `selection.predicate.populate(..)`, that shape.
/// `takeField` is the same call `let x = s.field` has always written; only this
/// position was not asking for it.
#[test]
fn a_self_method_on_a_field_takes_the_field_out() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Entity { pub name: String }\n\
         impl Entity {\n\
           pub fn into_name(self) -> String { self.name }\n\
           pub fn width(&self) -> usize { self.name.len() }\n\
         }\n\
         pub struct Pair { pub one: Entity, pub two: Entity }\n\
         pub fn name_of(pair: Pair) -> String { pair.one.into_name() }\n\
         pub fn width_of(pair: &Pair) -> usize { pair.one.width() }",
    )]);
    let taken = f.translated_method("lib.rs", "name_of");
    assert!(taken.contains("pair.takeField('one')"), "the field is taken out:\n{}", taken);
    assert!(taken.contains("pair.drop()"), "the struct is still the block's:\n{}", taken);

    // A `&self` method takes nothing, and the field is read where it is.
    let read = f.translated_method("lib.rs", "width_of");
    assert!(read.contains("pair.one.width()"), "{}", read);
    assert!(!read.contains("takeField"), "a borrow takes nothing:\n{}", read);
}

/// H7: a chain link whose PATTERN is refused still has its GUARD.
///
/// Rust tries the arm below when a guard fails, so a value the guard rejects
/// was never this arm's to refuse. Dropped, the hole ran for every value the
/// variant matched — including the ones the arm would never have run for — and
/// the comment beside it claimed the opposite.
#[test]
fn a_refused_link_keeps_its_guard() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         impl Drop for Token { fn drop(&mut self) { } }\n\
         pub enum Inner { X(Token), Y }\n\
         impl Drop for Inner { fn drop(&mut self) { } }\n\
         pub enum Held { One(Inner), Two(u32), Nothing }\n\
         pub struct Picker;\n\
         impl Picker {\n\
           pub fn pick(&self, h: Held, odd: bool) -> u32 {\n\
             match h {\n\
               Held::One(Inner::X(t)) if odd => t.n,\n\
               Held::Two(n) => n,\n\
               _ => 0,\n\
             }\n\
           }\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "pick");
    assert!(ts.contains("unsupported("), "the pattern is still refused:\n{}", ts);
    let hole = ts.find("unsupported(").expect("refused");
    let guard = ts.find("odd").expect("the guard is written");
    assert!(guard < hole, "the guard stands before the hole it opens:\n{}", ts);
    // And what it owes on its own throw path is the whole payload, because a
    // refused link declared none of the pattern's names: naming one would be a
    // `ReferenceError` in the `catch`.
    assert!(!ts.contains("t.drop()"), "nothing declared `t` here:\n{}", ts);
}

/// R1: a `?` keeps its null test unless its OWN operand's value is a hole.
///
/// The counter it used to read is global, so a refusal buried in a callback the
/// operand passes was read as "the operand IS a hole" and the test went with
/// it. `storage-common/planner.ts`'s `build_ineq_first_plan` then bound `null`
/// and carried on computing where Rust answers `None`.
#[test]
fn a_hole_inside_the_operand_does_not_take_the_question_marks_test() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub fn pick(xs: Vec<u32>, ys: Vec<u32>) -> Option<u32> {\n\
             let v: u32 = xs.iter().find_map(|x| {\n\
                 if *x == 99 { let mut it = ys.clone().into_iter(); it.next() } else { Some(*x) }\n\
             })?;\n\
             Some(v + 1)\n\
         }\n",
    )]);
    let ts = f.translated_method("lib.rs", "pick");
    assert!(
        ts.contains("if (_r0 == null) return null;"),
        "the operand answered an `Option`; the hole is in a branch of the \
         callback it passed:\n{}",
        ts
    );
    assert!(ts.contains("unsupported("), "the hole is still written:\n{}", ts);
}

/// The other half of the same rule: where the operand's own value IS the hole
/// there is nothing to test, and the `?` stands for the name the hole left.
#[test]
fn a_hole_that_is_the_whole_operand_still_leaves_no_test_behind() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub fn wholly(ys: Vec<u32>) -> Option<u32> {\n\
             let mut it = ys.into_iter();\n\
             let v = it.next()?;\n\
             Some(v + 1)\n\
         }\n",
    )]);
    let ts = f.translated_method("lib.rs", "wholly");
    assert!(
        !ts.contains("== null"),
        "a hole answers `never`; testing it would read as though it might have \
         answered something:\n{}",
        ts
    );
}

/// S1: the transfer a refusal cleanup waits on is a fact about the FRAME, not a
/// mark on the value.
///
/// `count(rest)` takes the `Vec<Token>` by value, so by the time the second `?`
/// refuses the tokens are already released. The cleanup used to ask the array
/// whether it had been moved — `markMoved` is protected on `AkObject` and an
/// array is not one, so both reads answered `undefined`, the guard passed, and
/// the tokens were dropped a second time: `BUG: Token was dropped twice`.
#[test]
fn a_vec_handed_over_before_the_refusal_is_not_released_again() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         impl Drop for Token { fn drop(&mut self) {} }\n\
         pub fn pass(t: Token) -> Result<Token, String> { Ok(t) }\n\
         pub fn count(xs: Vec<Token>) -> Result<u32, String> { Ok(xs.len() as u32) }\n\
         pub fn f(rest: Vec<Token>, more: Vec<Token>) -> Result<u32, String> {\n\
             let _pair = (count(rest)?, more.into_iter().map(pass).collect::<Result<Vec<_>, _>>()?);\n\
             Ok(0)\n\
         }\n",
    )]);
    let ts = f.translated_method("lib.rs", "f");
    assert!(
        !ts.contains("(rest as any).isMoved"),
        "an array carries no move mark, so asking it was asking something that \
         always answered `nobody has taken it`:\n{}",
        ts
    );
    let set = ts.find("= true;").expect("a flag is set somewhere");
    let call = ts.find("count(rest)").expect("the consuming call is written");
    assert!(
        call < set,
        "the flag stands where the transfer is written, below the call that \
         performs it:\n{}",
        ts
    );
    assert!(
        ts.contains(") dropOwned(rest);"),
        "and the release still stands, under the flag:\n{}",
        ts
    );
}

/// R9: a refusal in the statement's OWN text releases the by-value parameters
/// the call it aborted would have taken.
///
/// There used to be two walks here, and only the one for a refusal in a HOIST
/// knew about parameters; the other skipped every name with no block ordinal.
/// `let _v = take2(held, <hole>);` therefore released neither of its two.
#[test]
fn a_refusal_in_the_statements_own_text_releases_its_parameters() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         impl Drop for Token { fn drop(&mut self) {} }\n\
         pub fn pass(t: Token) -> Result<Token, String> { Ok(t) }\n\
         pub fn take2(a: Token, b: Vec<Token>) -> u32 { a.n + b.len() as u32 }\n\
         pub fn f(held: Token, rest: Vec<Token>) -> u32 {\n\
             let _v = take2(held, rest.into_iter().map(pass).collect::<Result<Vec<_>, _>>().unwrap());\n\
             _v\n\
         }\n",
    )]);
    let ts = f.translated_method("lib.rs", "f");
    assert!(ts.contains("held.drop();"), "the first parameter:\n{}", ts);
    assert!(ts.contains("dropOwned(rest);"), "and the second:\n{}", ts);
}

/// S2: a refusal inside a consuming loop releases the element THIS turn holds.
///
/// The loop's own claim removes a binding it sees moved, and the tail release
/// starts after the current index — so the element already handed out was
/// reached by neither, and the collector reported it.
#[test]
fn a_refusal_inside_a_consuming_loop_releases_the_current_element() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         impl Drop for Token { fn drop(&mut self) {} }\n\
         pub fn pass(t: Token) -> Result<Token, String> { Ok(t) }\n\
         pub fn f(items: Vec<Vec<Token>>) -> u32 {\n\
             let mut total = 0;\n\
             for rest in items {\n\
                 let _v = rest.into_iter().map(pass).collect::<Result<Vec<_>, _>>().unwrap();\n\
                 total += 1;\n\
             }\n\
             total\n\
         }\n",
    )]);
    let ts = f.translated_method("lib.rs", "f");
    assert!(ts.contains("dropOwned(rest);"), "the element this turn holds:\n{}", ts);
    assert!(
        ts.contains(".slice(_at"),
        "and the tail the loop never handed out, which is a different set:\n{}",
        ts
    );
    assert!(
        !ts.contains("dropOwned(items);"),
        "the sequence itself is the LOOP's — it is aliased into `_seqN` and its \
         tail released from the loop's own `finally`, so releasing the name as \
         well drops every element the loop already handed out:\n{}",
        ts
    );
}
