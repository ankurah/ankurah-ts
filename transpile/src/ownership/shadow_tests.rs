//! U2: a name declared ABOVE a body, and a `let` inside it that shadows it.
//!
//! A function's parameters, a closure's, and the names an arm's pattern bound
//! are all declared above the body they are read in. Every move site in that
//! body used to be attributed to them, whatever stood between — so a top-level
//! `let` of the same name handed the outer value's release to the shadow's
//! move, and the outer value was released by nobody.

use crate::testing::Fixture;

const TOKEN: &str = "\
pub struct Token { pub n: i64 }\n\
impl Drop for Token { fn drop(&mut self) {} }\n\
";

fn emitted(rust: &str) -> (String, Vec<String>) {
    let mut fixture = Fixture::build(&[("lib.rs", &format!("{}{}", TOKEN, rust))]);
    let ts = fixture.emitted("lib.rs");
    (ts, fixture.messages())
}

/// The shape U2 names: the shadow is MOVED, and the parameter is not.
#[test]
fn a_shadow_that_is_moved_does_not_take_the_parameters_release_with_it() {
    let (ts, _) = emitted(
        "pub fn held(token: Token) -> i64 {\n\
           let token = Token { n: 2 };\n\
           drop(token);\n\
           1\n\
         }",
    );
    assert!(ts.contains("token_1.drop()"), "the shadow is released where Rust drops it:\n{}", ts);
    assert!(ts.contains("token.drop()"), "and the parameter still is too:\n{}", ts);
}

/// The same where the shadow is RETURNED, which is the move the outer name
/// least deserves to be credited with.
#[test]
fn a_shadow_that_is_returned_leaves_the_parameter_owned() {
    let (ts, _) = emitted(
        "pub fn swapped(t: Token) -> Token {\n\
           let t = Token { n: 9 };\n\
           t\n\
         }",
    );
    assert!(ts.contains("t.drop()"), "the parameter is released:\n{}", ts);
}

/// And a shadow that is KEPT changes nothing: both are the body's, and both are
/// released.
#[test]
fn a_shadow_that_is_kept_is_released_beside_what_it_shadows() {
    let (ts, _) = emitted(
        "pub fn both(token: Token) -> i64 {\n\
           let token = Token { n: 2 };\n\
           token.n\n\
         }",
    );
    assert!(ts.contains("token_1.drop()"), "the shadow:\n{}", ts);
    assert!(ts.contains("token.drop()"), "and what it shadows:\n{}", ts);
}

/// A CLOSURE's parameter is declared above its body in the same way.
#[test]
fn a_closure_parameter_a_let_shadows_is_still_the_closures() {
    let (ts, _) = emitted(
        "pub fn apply<F>(t: Token, f: F) -> i64 where F: Fn(Token) -> i64 { f(t) }\n\
         pub fn used() -> i64 {\n\
           apply(Token { n: 1 }, |token: Token| {\n\
             let token = Token { n: 2 };\n\
             drop(token);\n\
             1\n\
           })\n\
         }",
    );
    assert!(ts.contains("token_1.drop()"), "the shadow:\n{}", ts);
    assert!(ts.contains("token.drop()"), "and the closure's own parameter:\n{}", ts);
}

/// And so are the names a `match` arm's pattern bound: the analogous tuple
/// value-match leaked its first binding.
#[test]
fn an_arm_binding_a_let_shadows_is_still_the_arms() {
    let (ts, _) = emitted(
        "pub fn taken() -> i64 {\n\
           let pair = (Token { n: 1 }, Token { n: 2 });\n\
           match pair {\n\
             (a, b) => {\n\
               let a = Token { n: 3 };\n\
               drop(a);\n\
               b.n\n\
             }\n\
           }\n\
         }",
    );
    assert!(ts.contains("a_1.drop()"), "the shadow:\n{}", ts);
    assert!(ts.contains("a.drop()"), "and the binding it shadows:\n{}", ts);
    assert!(ts.contains("b.drop()"), "and the other binding:\n{}", ts);
}
