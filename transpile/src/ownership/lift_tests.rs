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

/// A `?` LEAVES THE FRAME, so a move written before one is conditional.
///
/// `is_place` answers yes for a `syn::Expr::Try` — right for "does this leave a
/// value to release", wrong for "can this leave the frame" — so
/// `Some(take2(t, o?))` emitted `const _r0 = o; if (_r0 == null) return null;
/// return take2(t, _r0);` and returned with `t` moved into an argument list the
/// call never reached.
#[test]
fn a_question_mark_after_a_move_makes_the_move_conditional() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         impl Drop for Token { fn drop(&mut self) {} }\n\
         pub fn take2(a: Token, b: u32) -> u32 { a.n + b }\n\
         pub fn f(t: Token, o: Option<u32>) -> Option<u32> { Some(take2(t, o?)) }",
    )]);
    let ts = f.translated_method("lib.rs", "f");
    assert!(ts.contains("t.drop();"), "the frame still owns it:\n{}", ts);
    let flag = ts.find("= true;").expect("a flag is set");
    let leave = ts.find("return null;").expect("the `?` leaves");
    assert!(leave < flag, "and the flag stands below the `?`:\n{}", ts);
}

/// A tuple or array ELEMENT is on the same lift and flag plan as a call's
/// argument.
///
/// `(token, o.unwrap())` set the flag, handed `token` to the array literal, and
/// then threw: the token was in a literal nothing finished building, and the
/// flag said somebody else owned it.
#[test]
fn an_aggregate_lifts_a_throwing_element_above_the_flag() {
    for built in ["(token, o.unwrap())", "[token, mk(o.unwrap())]"] {
        let mut f = Fixture::build(&[(
            "lib.rs",
            &format!(
                "pub struct Token {{ pub n: u32 }}\n\
                 impl Drop for Token {{ fn drop(&mut self) {{}} }}\n\
                 pub fn mk(n: u32) -> Token {{ Token {{ n }} }}\n\
                 pub fn f(token: Token, o: Option<u32>) -> u32 {{ let _p = {}; 0 }}",
                built
            ),
        )]);
        let ts = f.translated_method("lib.rs", "f");
        let flag = ts.find("= true;").expect("a flag is set");
        let throw = ts.find("unwrap()").expect("the throw is written");
        assert!(throw < flag, "the throw stands above the flag:\n{}", ts);
        assert!(ts.contains("token.drop()"), "and the frame still owns it:\n{}", ts);
    }
}

/// A move NESTED inside an operand is reached by the same rule.
///
/// The aggregate arms of the move walk recursed with a plain position, so
/// `Box3 { a: Some(x), k, c: o.unwrap() }` covered `k` and not `x`: `x` was
/// handed over unconditionally and released by nobody when the `unwrap` threw.
#[test]
fn a_move_nested_in_an_operand_is_conditional_too() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         impl Drop for Token { fn drop(&mut self) {} }\n\
         pub struct Box3 { pub a: Option<Token>, pub k: Token, pub c: u32 }\n\
         pub fn f(x: Token, k: Token, o: Option<u32>) -> Box3 { \
         Box3 { a: Some(x), k, c: o.unwrap() } }",
    )]);
    let ts = f.translated_method("lib.rs", "f");
    assert!(ts.contains("x.drop();"), "the nested move is the frame's:\n{}", ts);
    assert!(ts.contains("k.drop();"), "and so is the one written as a field:\n{}", ts);
}

/// A droppable TEMPORARY built before a throwing operand is the frame's, with
/// no flagged local anywhere in the statement.
///
/// `Box4 { a: mk(x), b: mk(k), c: o.unwrap() }` emitted
/// `new Box4(mk(x), mk(k), (o ?? throw))`, and both `mk` results were owned by
/// nobody when the `unwrap` threw — which Rust drops while it unwinds. The
/// sibling with `b: k` was right only because `k` is a flagged local.
#[test]
fn a_temporary_built_before_a_throw_is_released() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         impl Drop for Token { fn drop(&mut self) {} }\n\
         pub struct Box4 { pub a: Token, pub b: Token, pub c: u32 }\n\
         pub fn mk(n: u32) -> Token { Token { n } }\n\
         pub fn f(o: Option<u32>) -> Box4 { Box4 { a: mk(1), b: mk(2), c: o.unwrap() } }",
    )]);
    let ts = f.translated_method("lib.rs", "f");
    assert_eq!(
        ts.matches("dropOwned(_b").count(),
        2,
        "both temporaries are released if the throw is reached:\n{}",
        ts
    );
}

/// And a name is never lifted into another name.
///
/// CC6: `evaluating` asks `evaluates_quietly` of the RUST expression while the
/// placement asks the TEXT, so `Vec::new()` in a later field was "can throw"
/// for one and "a literal, do not lift" for the other — a flag with nothing
/// between it and the call, and `const _b1 = inner;`, a name aliasing a name.
#[test]
fn a_bare_name_is_not_lifted_into_another_name() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         impl Drop for Token { fn drop(&mut self) {} }\n\
         pub fn take2(a: Token, b: u32) -> u32 { a.n + b }\n\
         pub fn f(t: Token, inner: u32, o: Option<u32>) -> u32 { \
         let _x = take2(t, inner); o.unwrap() }",
    )]);
    let ts = f.translated_method("lib.rs", "f");
    assert!(!ts.contains("= inner;"), "a name already has a name:\n{}", ts);
}

/// A place the PORT writes as a CALL makes a move beside it conditional.
///
/// `handle.n` on a value behind a `Deref` is emitted as `handle.deref().n`, and
/// `deref()` on a value somebody dropped throws. Rust cannot panic reading a
/// field, so the disposition called the move unconditional and
/// `Event { t: token, n: handle.n }` left the token released by nobody. The
/// placement rule has asked the emitted text this since W2; the disposition
/// asks it now too, before the text exists.
#[test]
fn an_auto_deref_beside_a_move_makes_the_move_conditional() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "use std::sync::Arc;\n\
         pub struct Token { pub n: u32 }\n\
         impl Drop for Token { fn drop(&mut self) {} }\n\
         pub struct Inner { pub n: u32 }\n\
         pub struct Handle { inner: Arc<Inner> }\n\
         impl std::ops::Deref for Handle { type Target = Inner; \
         fn deref(&self) -> &Inner { &self.inner } }\n\
         pub struct Event { pub t: Token, pub n: u32 }\n\
         pub fn f(token: Token, handle: Handle) -> Event { \
         Event { t: token, n: handle.n } }",
    )]);
    let ts = f.translated_method("lib.rs", "f");
    assert!(ts.contains("token.drop()"), "the frame still owns it:\n{}", ts);
    let flag = ts.find("_moved0 = true;").expect("a flag is set");
    let deref = ts.find("handle.deref()").expect("the port writes the deref");
    assert!(deref < flag, "and the flag stands below it:\n{}", ts);
}

/// A struct literal's fields are EVALUATED in the order the literal writes
/// them, and handed to the constructor in declaration order.
///
/// `Reordered { n: value.unwrap(), token }` was translated in declaration
/// order, so `token` was handed over before the `unwrap` Rust runs first and a
/// `None` left it owned by nobody.
#[test]
fn a_struct_literal_evaluates_its_fields_in_source_order() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         impl Drop for Token { fn drop(&mut self) {} }\n\
         pub struct Reordered { pub token: Token, pub n: u32 }\n\
         pub fn f(token: Token, value: Option<u32>) -> Reordered { \
         Reordered { n: value.unwrap(), token } }",
    )]);
    let ts = f.translated_method("lib.rs", "f");
    let throw = ts.find("throw new Error").expect("the unwrap is written");
    let build = ts.find("new Reordered(").expect("the constructor is written");
    assert!(throw < build, "the first field is evaluated first:\n{}", ts);
    assert!(
        ts.contains("new Reordered(token, _b"),
        "and the values reach the constructor in declaration order:\n{}",
        ts
    );
    assert!(ts.contains("token.drop()"), "the frame owns the token until then:\n{}", ts);
}

/// An assignment to a local that a branch may already have handed away
/// releases the old value under its flag, and puts the flag back.
///
/// The assignment gives the local a value nobody has taken. Reported instead of
/// written, six of `storage-common/planner.rs`'s `low = candidate` sites left
/// the value the local held released by nobody.
#[test]
fn an_assignment_to_a_flagged_local_releases_and_resets() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        "pub struct Token { pub n: u32 }\n\
         impl Drop for Token { fn drop(&mut self) {} }\n\
         pub struct Pair { pub a: Token, pub b: u32 }\n\
         pub fn mk(n: u32) -> Token { Token { n } }\n\
         pub fn f(mut low: Token, o: Option<u32>, again: bool) -> u32 {\n\
             if again { low = mk(2); }\n\
             let p = Pair { b: o.unwrap(), a: low };\n\
             p.b\n\
         }",
    )]);
    let ts = f.translated_method("lib.rs", "f");
    assert!(ts.contains(") low.drop();"), "the old value is released:\n{}", ts);
    assert!(ts.contains(" = false;\n"), "and the flag goes back:\n{}", ts);
}
