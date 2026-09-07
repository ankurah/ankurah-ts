//! An arm's BODY: what it is written as, and what it owns while it runs.
//!
//! An arm is an arrow function in the runtime's keyed match and a block in
//! every chain form, so the body is written for the position the arm stands in
//! — and where the enum handed its payload over, the arm owns what the pattern
//! named and releases it however the arm is left.

use super::arms::body_of_an_arm;
use crate::body::BodyTranslator;

/// An arm's body, and what the arm owes around it.
pub(super) struct Body {
    pub(super) body: String,
    pub(super) lifted: Vec<crate::ownership::Hoist>,
    pub(super) owned: Vec<crate::ownership::Owned>,
    pub(super) flags: String,
    /// Is `body` one EXPRESSION whose value the arm's arrow still has to hand
    /// back, or a run of statements that has already done it?
    pub(super) value: bool,
    /// Does every path out of the arm leave the arrow — so that a chain need
    /// write no jump after it?
    pub(super) leaves: bool,
}

/// The body both forms of arm share: what it declares, what it owns, and what
/// it says about leaving early.
pub(super) fn translate_body(
    arm: &syn::Arm,
    declared: &[String],
    takes: crate::ownership::scrutinee::Takes,
    t: &BodyTranslator,
    match_expr: &syn::ExprMatch,
    position: super::Position,
    produces: bool,
    unknown_bindings: crate::ownership::Drops,
) -> Body {
    // Where the enum handed its payload over, the arm owns what the pattern
    // named and releases it however the arm is left.
    //
    // H12: a name bound out of a tuple MEMBER has no type the payload walk can
    // give it — the walk is asked with the SUBJECT's type and a pattern nested
    // inside a member comes back with no members at all — and the arm owns it
    // all the same. `Drops::Cascade` is the same answer a `Result` arm's bound
    // payload gets: the arm holds a value the side read out of the enum
    // whatever its type turns out to be, and `dropOwned` releases it by its
    // runtime shape. Every other arm keeps `Drops::Unknown`, which releases
    // nothing and says so.
    let names: Vec<String> = match takes {
        crate::ownership::scrutinee::Takes::Payload => declared.to_vec(),
        crate::ownership::scrutinee::Takes::Nothing => Vec::new(),
    };
    let owned = t.claim_bindings_as(
        &names,
        &|name| t.types.as_ref().and_then(|tc| tc.borrow().lookup(name)),
        unknown_bindings,
        std::slice::from_ref(&syn::Stmt::Expr(arm.body.as_ref().clone(), None)),
    );
    // An arm is an arrow function, so what the arm's own expression lifted out
    // of itself stays inside it: the declaration names values the arm's payload
    // produced, which do not exist outside. A block body is written as the
    // arrow's own statements, with the `return` on its tail; as an
    // immediately-called function it computed the arm's value and threw it
    // away, and a `return` written inside it left the inner function rather
    // than the enclosing one.
    //
    // K2: where the match hands a value back, the body is written for the
    // position that WANTS one, so a nested match — `Expr::Placeholder =>
    // match values.next() { Some(v) => Ok(..), None => Err(..) }` — puts a
    // `return` on each of its own branches instead of standing there as a
    // statement whose value nobody takes. `ankql/ast.ts`'s
    // `Expr.populateRecursive` answered `undefined` for exactly that arm.
    // P1: an arm's value IS the match's value, so what the position wants of the
    // match it wants of every arm — re-keyed onto the arm's own span, because an
    // expectation is matched by the span of the expression it was written for.
    // Without it `match l { Literal::Bool(b) => vec![*b as u8], .. }` built a
    // `number[]` where every other arm of the same match answers a
    // `Uint8Array`, and the function's declared `Vec<u8>` said so one level up.
    // Live at `core/collation.ts` and `core/value/collatable.ts`.
    let want = t
        .expectation_for(&syn::Expr::Match(match_expr.clone()))
        .or_else(|| match position {
            super::Position::Returning => t.fn_return.clone(),
            super::Position::Statement => None,
        });
    let ((body, value), lifted) = t.with_own_hoists(|| {
        t.expecting(&arm.body, want.as_ref(), || body_of_an_arm(&arm.body, produces, t))
    });
    let body = body.trim_end().to_string();
    // Where the match hands a value back, EVERY path out of the arm hands one
    // back too — that is what Rust's type for the arm says — so the lowering
    // wrote a `return` on each of them and the arm leaves. Where the match's
    // own value is `()`, nothing was returned and the arm leaves only where
    // the Rust does: an `if` with NO `else` runs on when its test fails, which
    // reading the last line of the text backwards could not tell (K2).
    let leaves = produces || crate::control_flow::form::always_leaves(&arm.body);
    // An arm is an arrow function, so a `?` inside one returns from the arm.
    // Where the match is the enclosing function's value that is exactly right —
    // the arm's `Result` is what the function returns — and where it is a
    // statement it is not, and nobody sees the error. `leaves_the_loop` routes
    // such a match through the sentinel, which sets `jump_as_value`; anything
    // still here has no route.
    if position == super::Position::Statement
        && !t.jump_as_value.get()
        && super::leaves_the_function(&arm.body)
    {
        t.report_match_gap(
            match_expr,
            "an arm leaves early, and the arm is an arrow function whose `return` leaves the \
             arm rather than the function, so nobody sees the error it left with",
        );
    }
    // An arm is an arrow function, so a local this arm hands away sets its drop
    // flag here — the same line the enclosing block would have written had the
    // arm been a statement of it.
    let flags = t.flag_sets_that_run(t.flag_sets_for(&arm.body), &body);
    Body { body, lifted, owned, flags, value, leaves }
}

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

    /// P1: an arm's value IS the match's value, so what the position wants of
    /// the match it wants of every arm. The keyed form set no expectation on
    /// its arms at all, so a `vec![b as u8]` inside one built a `number[]`
    /// where the same expression as the function's tail builds a `Uint8Array` —
    /// and where every other arm of the same match answers one. Live at
    /// `core/collation.ts` and `core/value/collatable.ts`.
    #[test]
    fn an_arm_is_written_for_what_the_match_is_expected_to_answer() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub enum Lit { Bool(bool), Empty }\n\
             pub fn to_bytes(l: &Lit) -> Vec<u8> {\n\
               match l { Lit::Bool(b) => vec![*b as u8], Lit::Empty => Vec::new() }\n\
             }",
        )]);
        let ts = f.translated_method("lib.rs", "to_bytes");
        assert!(ts.contains("new Uint8Array([Number(b)])"), "{}", ts);
        assert!(!ts.contains("return [Number(b)]"), "{}", ts);
    }

    /// And a STATEMENT match wants nothing of its arms, so nothing is asked of
    /// them: an expectation left standing there would convert a value nobody
    /// takes.
    #[test]
    fn a_statement_match_asks_nothing_of_its_arms() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub enum Lit { Bool(bool), Empty }\n\
             pub fn count(l: &Lit) -> u32 {\n\
               let mut n = 0u32;\n\
               match l { Lit::Bool(_) => { n = 1; }, Lit::Empty => { n = 2; } }\n\
               n\n\
             }",
        )]);
        let ts = f.translated_method("lib.rs", "count");
        assert!(!ts.contains("Uint8Array"), "{}", ts);
    }
}
