//! What a call HANDS OVER is a local of the body it hands it to. O1, L3.
//!
//! For: Rust drops a by-value closure parameter at the end of every invocation
//! — on the normal return and while an unwind passes through — because it IS a
//! local of the closure's body, exactly as a function's parameter is a local of
//! the function's. The port released none of them, so a generated callback
//! handed an element by value stored it nowhere and dropped nothing. The same
//! rule reaches a `match` arm: a name an arm binds by value is a local of that
//! arm, whatever the subject's shape.

use crate::testing::Fixture;

const TOKEN: &str = "pub struct Token(pub u32);\n\
                     impl Drop for Token { fn drop(&mut self) { } }\n";

fn body(rust: &str, method: &str) -> String {
    let mut f = Fixture::build(&[("lib.rs", &format!("{}{}", TOKEN, rust))]);
    f.translated_method("lib.rs", method)
}

/// The element `position` hands to its closure is the closure's, and the
/// closure releases it however the invocation ends.
#[test]
fn a_by_value_closure_parameter_is_released_at_the_end_of_the_invocation() {
    let ts = body(
        "pub fn position_of(tokens: Vec<Token>, want: u32) -> Option<usize> {\n\
           tokens.into_iter().position(|token| token.0 == want)\n\
         }",
        "position_of",
    );
    assert!(ts.contains("iterPositionOwned("), "the owned terminal is what is written:\n{}", ts);
    assert!(
        ts.contains("} finally {") && ts.contains("token.drop();"),
        "the callback releases what it was handed, however it is left:\n{}",
        ts
    );
}

/// A closure that hands its parameter on — to a callee, or as its own answer —
/// releases nothing, exactly as a function's parameter is not released where
/// every path hands it on.
#[test]
fn a_by_value_parameter_the_closure_hands_on_is_released_by_nothing() {
    let ts = body(
        "pub fn first_kept(tokens: Vec<Token>) -> Option<Token> {\n\
           tokens.into_iter().reduce(|a, b| { drop(b); a })\n\
         }",
        "first_kept",
    );
    assert!(ts.contains("iterReduceOwned("), "{}", ts);
    assert!(ts.contains("b.drop();"), "the body's own `drop(b)` stands:\n{}", ts);
    assert!(!ts.contains("a.drop();"), "`a` is the closure's answer:\n{}", ts);
    assert!(!ts.contains("finally"), "nothing is owed, so no scope is opened:\n{}", ts);
}

/// A parameter written `&T` is somebody else's, and Rust's match ergonomics
/// make every name a pattern binds UNDER one a borrow too: `xs.iter().map(|(f,
/// _)| ..)` hands the closure a reference to the pair and moves neither half.
#[test]
fn a_borrowed_closure_parameter_owes_nothing() {
    let ts = body(
        "pub fn names(pairs: &Vec<(String, Token)>) -> Vec<String> {\n\
           pairs.iter().map(|(f, _)| f.clone()).collect()\n\
         }",
        "names",
    );
    assert!(!ts.contains("finally"), "a borrow releases nothing:\n{}", ts);
    assert!(!ts.contains("_.drop()"), "and names nothing to release it under:\n{}", ts);
}

/// A parameter pattern that takes the value apart binds each half by value, and
/// each half is a local of the body.
#[test]
fn each_name_a_parameter_pattern_binds_is_claimed_on_its_own() {
    let ts = body(
        "pub fn tags(pairs: Vec<(Token, Token)>) -> Vec<u32> {\n\
           pairs.into_iter().map(|(one, two)| one.0 + two.0).collect()\n\
         }",
        "tags",
    );
    assert!(ts.contains("one.drop();"), "the first half is released:\n{}", ts);
    assert!(ts.contains("two.drop();"), "and so is the second:\n{}", ts);
}

/// A closure's expression body is its VALUE, so a field read in it hands the
/// field to the caller and leaves the rest of the struct where it was — the
/// same partial move a block's last expression already wrote.
#[test]
fn a_field_read_in_a_closures_expression_body_takes_the_field_out() {
    let mut f = Fixture::build(&[(
        "lib.rs",
        &format!(
            "{}pub struct Holder {{ pub item: Token, pub tag: u32 }}\n\
             pub fn items(hs: Vec<Holder>) -> Vec<Token> {{\n\
               hs.into_iter().map(|h| h.item).collect()\n\
             }}",
            TOKEN
        ),
    )]);
    let ts = f.translated_method("lib.rs", "items");
    assert!(ts.contains("h.takeField('item')"), "the field comes OUT of the struct:\n{}", ts);
    assert!(ts.contains("h.drop();"), "and the rest of it is released here:\n{}", ts);
}

/// L3: a name an arm binds by value is a local of that arm, whatever the
/// SUBJECT's shape. Claimed only where the pattern bound the WHOLE subject, a
/// tuple subject whose arm binds both operands released neither
/// (`storage-common`'s two comparators, six arms).
#[test]
fn an_arm_of_a_value_match_owns_every_name_it_binds() {
    let ts = body(
        "pub fn total(a: Token, b: Token) -> u32 {\n\
           match (a, b) { (x, y) => x.0 + y.0 }\n\
         }",
        "total",
    );
    assert!(ts.contains("x.drop();"), "the first operand is released:\n{}", ts);
    assert!(ts.contains("y.drop();"), "and so is the second:\n{}", ts);
}

/// And an arm that hands its binding on still releases nothing, while the
/// position its pattern did not name is still owed.
#[test]
fn an_arm_that_hands_its_binding_on_releases_it_nowhere() {
    let ts = body(
        "pub fn keep_first(a: Option<Token>, b: Option<Token>) -> Option<Token> {\n\
           match (a, b) { (Some(x), _) => Some(x), (None, other) => other }\n\
         }",
        "keep_first",
    );
    assert!(!ts.contains("x.drop()"), "`x` is the arm's answer:\n{}", ts);
    assert!(!ts.contains("other.drop()"), "and so is `other`:\n{}", ts);
    assert!(ts.contains("dropOwned(_v[1])"), "the position nothing named is owed:\n{}", ts);
}

/// A match that only READS its subject hands nothing over, so its arms' names
/// are borrows and own nothing.
#[test]
fn a_borrowing_match_claims_none_of_its_arms_names() {
    let ts = body(
        "pub fn total(pair: &(Token, Token)) -> u32 {\n\
           match pair { (x, y) => x.0 + y.0 }\n\
         }",
        "total",
    );
    assert!(!ts.contains(".drop()"), "nothing here is anybody's to release:\n{}", ts);
}

/// A claim the port cannot honour is WITHDRAWN, not written: where the body
/// moved a field out of the value and the port wrote a plain read, the struct's
/// cascade still reaches that field, so releasing the struct here would release
/// it a second time. Two corpus sites — `peer_subscription/server.rs`'s
/// `update.items` and `storage-common`'s `sorting.rs` `h.item`.
#[test]
fn a_claim_that_would_double_drop_a_field_is_withdrawn_and_reported() {
    let mut f = crate::testing::Fixture::build(&[(
        "lib.rs",
        &format!(
            "{}pub struct Holder {{ pub items: Vec<Token>, pub tag: u32 }}\n\
             pub fn counts(hs: Vec<Holder>) -> Vec<usize> {{\n\
               hs.into_iter().map(|h| h.items.into_iter().count()).collect()\n\
             }}",
            TOKEN
        ),
    )]);
    let ts = f.translated_method("lib.rs", "counts");
    assert!(!ts.contains("h.drop()"), "the claim is withdrawn:\n{}", ts);
    assert!(
        f.messages().iter().any(|m| m.contains("would release that field a second time")),
        "and the site says so: {:?}",
        f.messages()
    );
}
