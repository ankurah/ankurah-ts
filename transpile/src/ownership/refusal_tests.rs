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
