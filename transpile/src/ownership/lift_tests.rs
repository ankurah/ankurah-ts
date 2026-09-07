//! What a LIFT owes when the call it was lifted for is never written.
//!
//! For: an argument the port lifts above a move flag holds a value Rust had not
//! built yet, so nobody owns it until the call takes it. The flag that reports
//! that transfer is a claim about text the port WROTE, and where the port
//! refused the call — the terminal of the chain the lift stood in has no
//! construction, so the whole expression came out as one hole — the claim is
//! false and the `finally` believed it.
//!
//! And where the call IS written, the flag has to stand below every operand
//! still to be evaluated, including the ones the port itself added: a field
//! read cannot panic in Rust and `this.deref().n` is a call.

use crate::testing::Fixture;

/// The corpus shape, in one crate: `storage-indexeddb/collection.ts` lifted
/// `order_by_spill.clone()` for a `top_k` whose `collect` the port refused, set
/// the lift's flag immediately above the hole, and released the clone nowhere.
fn refused_callee() -> String {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         #[derive(Clone)]\n\
         pub struct Spill { pub n: u32 }\n\
         pub struct Rows { pub n: u32 }\n\
         impl Rows { pub fn top_k(self, spill: Spill, k: usize) -> Vec<Token> { Vec::new() } }\n\
         pub fn tally<T>(x: T) -> u32 { 0 }\n\
         pub fn refused(rows: Rows, spill: Spill, limit: Option<u32>, leave: bool) -> u32 {\n\
           let held = rows;\n\
           if leave { return 0; }\n\
           match limit {\n\
             Some(k) => tally(held.top_k(spill.clone(), k as usize).into_iter().map(|t| t).collect()),\n\
             None => 0,\n\
           }\n\
         }",
    )]);
    f.translated_method("lib.rs", "refused")
}

#[test]
fn a_lift_for_a_refused_call_is_released_however_the_hole_is_left() {
    let ts = refused_callee();
    assert!(ts.contains("unsupported("), "the collect was expected to refuse:\n{}", ts);
    assert!(
        ts.contains("dropOwned(_b1);"),
        "the clone the port lifted for `top_k` is released, because no `top_k` was \
         written to take it:\n{}",
        ts
    );
    assert!(
        !ts.contains("if (!_moved2)"),
        "and the flag that said the call had taken it is gone with the transfer it \
         reported:\n{}",
        ts
    );
}

#[test]
fn no_move_flag_is_set_above_a_hole_that_aborts_the_transfer() {
    let ts = refused_callee();
    let set = ts.lines().position(|l| l.trim().starts_with("_moved") && l.contains("= true;"));
    assert!(
        set.is_none(),
        "K3 for an ARM: `held` is handed to a `top_k` the port never wrote, so no flag \
         says it was:\n{}",
        ts
    );
    assert!(
        ts.contains("held.drop();"),
        "and the block releases it unguarded, there being no flag left to read:\n{}",
        ts
    );
}

/// X2's sibling site: the LAST lift of a call carries no flag, because nothing
/// between it and the call can throw — but the obligation is the lift's, not
/// the flag's, and a call the port never wrote takes nothing.
#[test]
fn the_last_lift_of_a_refused_call_is_released_too() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         #[derive(Clone)]\n\
         pub struct Spill { pub n: u32 }\n\
         pub fn refused(tokens: Vec<Token>, spill: Spill, limit: Option<u32>, leave: bool) -> u32 {\n\
           let held = tokens;\n\
           if leave { return 0; }\n\
           match limit {\n\
             Some(k) => { let _ = k; held.into_iter().zip(vec![spill.clone()]).collect() }\n\
             None => 0,\n\
           }\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "refused");
    assert!(ts.contains("unsupported("), "the collect was expected to refuse:\n{}", ts);
    assert!(
        ts.contains("dropOwned(_b1);"),
        "the array the port lifted is released even though it never carried a flag:\n{}",
        ts
    );
}

/// The other half of the same rule: where the call IS written, the lift stands
/// there to be taken and nothing is released behind the callee's back.
#[test]
fn a_lift_a_written_call_takes_keeps_its_flag() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         #[derive(Clone)]\n\
         pub struct Spill { pub n: u32 }\n\
         pub fn take3(t: Token, s: Spill, k: usize) -> u32 { 0 }\n\
         pub fn kept(token: Token, spill: Spill, leave: bool) -> u32 {\n\
           if leave { return 0; }\n\
           take3(token, spill.clone(), 3usize)\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "kept");
    assert!(!ts.contains("unsupported("), "nothing here refuses:\n{}", ts);
    assert!(
        ts.contains("_moved0 = true;") && ts.contains("if (!_moved0) token.drop();"),
        "the token's flag is still written and still read:\n{}",
        ts
    );
}

/// W2: `self.n` is a place in Rust and `this.deref().n` in the port, and
/// `deref()` throws where the value it reaches is gone. So the flag stands
/// below it, which is what U3 asks of every other evaluand.
#[test]
fn a_place_the_port_writes_as_a_call_stands_above_the_flag() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "use std::ops::Deref;\n\
         pub struct Token { pub n: u32 }\n\
         pub struct Inner { pub n: u32 }\n\
         pub struct Handle(pub Option<Inner>);\n\
         impl Deref for Handle {\n\
           type Target = Inner;\n\
           fn deref(&self) -> &Inner { self.0.as_ref().unwrap() }\n\
         }\n\
         pub struct Event { pub token: Token, pub n: u32 }\n\
         impl Handle {\n\
           pub fn make(&self, token: Token, leave: bool) -> u32 {\n\
             if leave { return 0; }\n\
             let e = Event { token, n: self.n };\n\
             e.n\n\
           }\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "make");
    let deref = ts.find("this.deref().n").expect(&ts);
    let flag = ts.find("_moved0 = true;").expect(&ts);
    assert!(
        deref < flag,
        "the deref the port wrote is evaluated before the flag claims the constructor \
         has the token:\n{}",
        ts
    );
    assert!(
        ts.contains("const _b1 = this.deref().n;"),
        "and it is evaluated by being lifted, not by being moved:\n{}",
        ts
    );
}

/// The rule reads what the port wrote, so it must not read an arrow's
/// parameter list or a string's characters as a call: lifting a closure would
/// name it above the flag for nothing, and lifting a literal writes
/// `const _b2 = 'a(b)';`.
#[test]
fn a_closure_and_a_literal_are_not_lifted_for_their_parentheses() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         pub fn take_with(t: Token, f: impl Fn(u32) -> u32, label: &str) -> u32 { 0 }\n\
         pub fn quiet(token: Token, leave: bool) -> u32 {\n\
           if leave { return 0; }\n\
           take_with(token, |x| x + 1, \"a(b)\")\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "quiet");
    assert!(
        !ts.contains("const _b"),
        "an arrow's parentheses hold parameters and a literal's are characters:\n{}",
        ts
    );
}

// ── X5: ownership during left-to-right argument evaluation ────────────

/// X5: `take2(token, o.unwrap())` moves `token` on every path the SOURCE has,
/// so the disposition read straight-line and the block wrote no release at all
/// — and `unwrap` on a `None` throws with the token handed to nobody, which
/// Rust drops while it unwinds.
#[test]
fn a_move_with_a_throwing_argument_after_it_is_conditional() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         pub fn take2(t: Token, n: u32) -> u32 { t.n + n }\n\
         pub fn probe(t: Token, o: Option<u32>) -> u32 { take2(t, o.unwrap()) }",
    )]);
    let ts = f.translated_method("lib.rs", "probe");
    let lift = ts.find("const _b1 =").expect(&ts);
    let flag = ts.find("_moved0 = true;").expect(&ts);
    assert!(lift < flag, "the throwing argument is evaluated above the flag:\n{}", ts);
    assert!(
        ts.contains("if (!_moved0) t.drop();"),
        "and the block releases the token on the path the throw takes:\n{}",
        ts
    );
}

/// The same for a field of a STRUCT LITERAL, which is ankql's shape: a `?` in a
/// later field leaves the frame before the earlier field is handed over.
#[test]
fn a_struct_field_moved_before_a_later_question_mark_is_conditional() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Op { pub n: u32 }\n\
         pub struct Oops;\n\
         pub struct Pair { pub op: Op, pub n: u32 }\n\
         pub fn fallible(f: bool) -> Result<u32, Oops> { Ok(1) }\n\
         pub fn probe(op: Op, fail: bool) -> Result<Pair, Oops> {\n\
           Ok(Pair { op, n: fallible(fail)? })\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "probe");
    assert!(
        ts.contains("if (!_moved0) op.drop();"),
        "the `?` can leave before the literal is built, so the move is flagged:\n{}",
        ts
    );
}

/// And nothing changes where the operands after the move cannot throw: a
/// literal builds out of nothing.
#[test]
fn a_move_with_only_literals_after_it_stays_unconditional() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         pub fn take2(t: Token, n: u32) -> u32 { t.n + n }\n\
         pub fn probe(t: Token) -> u32 { take2(t, 3) }",
    )]);
    let ts = f.translated_method("lib.rs", "probe");
    assert!(
        !ts.contains("_moved") && !ts.contains("t.drop()"),
        "the token is gone on every path, so the block owes nothing:\n{}",
        ts
    );
}

/// V5: a value the port writes as a LITERAL is not lifted, because lifting it
/// takes it out of the position that typed it — `const _b2 = [];` is `any[]`,
/// which `noImplicitAny` reports twice at every such site.
#[test]
fn a_literal_operand_is_not_lifted_out_of_the_position_that_types_it() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Op { pub n: u32 }\n\
         pub struct Holder { pub op: Op, pub items: Vec<u32>, pub tail: Option<u32> }\n\
         pub fn probe(op: Op, o: Option<u32>) -> Holder {\n\
           Holder { op, items: Vec::new(), tail: Some(o.unwrap()) }\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "probe");
    assert!(
        !ts.contains("= [];"),
        "the empty vector stays in the field that says what it holds:\n{}",
        ts
    );
    assert!(
        ts.contains("if (!_moved0) op.drop();"),
        "and the `unwrap` after it still makes the move conditional:\n{}",
        ts
    );
}
