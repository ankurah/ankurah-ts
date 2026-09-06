//! What `patterns.rs` writes, checked against the shapes that went wrong.
//!
//! Apart from the writer because both modules are read on their own: the writer
//! is a decision table a reader follows top to bottom, and these are the cases
//! that decided it — a const pattern, and an or-pattern whose alternatives take
//! their names out of different places.

#[cfg(test)]
mod const_pattern_tests {
    use crate::testing::Fixture;

    /// A const pattern binds NOTHING: Rust compares the subject against the
    /// const's value. Read as a binding, the arm owned a value nothing
    /// declared — `match p { ORIGIN => true, _ => false }` released `oRIGIN`,
    /// an identifier the emitted file never introduces, and the arm the hole
    /// had replaced still carried it.
    ///
    /// PREMISE CHANGED 2026-09-05 (step 9a slice 2, H8): the refusal used to
    /// stand in the TEST — `if (unsupported(..))`. D2's rule is that a hole
    /// never stands in a condition: the branch's own bindings and releases then
    /// sit under a `never`, and every reader of a test has to know that
    /// spelling to read it. The test is `true` and the refusal is the first
    /// statement of the branch.
    #[test]
    fn a_const_pattern_binds_nothing_and_releases_nothing() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct Point { pub x: i32 }\n             impl Drop for Point { fn drop(&mut self) {} }\n             pub const ORIGIN: Point = Point { x: 0 };\n             pub fn at_origin(p: Point) -> bool { match p { ORIGIN => true, _ => false } }",
        )]);
        let ts = f.translated_method("lib.rs", "at_origin");
        assert!(!ts.contains("if (unsupported("), "the hole is not in the test:\n{ts}");
        assert!(ts.contains("unsupported("), "the branch refuses:\n{ts}");
        assert!(!ts.contains("oRIGIN"), "and it declares no binding:\n{ts}");
        // The subject is still the body's, released where nothing took it.
        assert!(ts.contains("p.drop()"), "{ts}");
    }

    /// The same for a const the port compares by value, which is not a hole:
    /// no binding there either, and the arms below it are reachable.
    #[test]
    fn a_primitive_const_pattern_is_a_comparison() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub const LIMIT: i32 = 5;\n             pub fn at_limit(n: i32) -> bool { match n { LIMIT => true, _ => false } }",
        )]);
        let ts = f.translated_method("lib.rs", "at_limit");
        assert!(ts.contains("n === LIMIT"), "{ts}");
        assert!(!ts.contains("const lIMIT"), "{ts}");
    }
}

#[cfg(test)]
mod or_pattern_tests {
    use crate::testing::Fixture;

    /// PREMISE CHANGED 2026-09-05 (fixpass6 item 4, D2): the alternatives used
    /// to be compared by POSITION, so `(Side::Property(p), Side::Literal(l)) |
    /// (Side::Literal(l), Side::Property(p))` — the very shape an or-pattern is
    /// for, and the one `core/src/reactor/watcherset.rs:171` writes — was
    /// refused. Rust requires the same SET of names, not the same order, and
    /// each name is looked up by name in every alternative now.
    #[test]
    fn an_or_pattern_binding_its_names_in_a_different_order_is_written() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct Lit { pub n: u32 }\n\
             pub struct Path { pub s: String }\n\
             pub enum Side { Literal(Lit), Property(Path) }\n\
             pub fn pair(left: Side, right: Side) -> u32 {\n\
               if let (Side::Property(p), Side::Literal(l)) | (Side::Literal(l), Side::Property(p)) = (left, right) {\n\
                 l.n + p.s.len() as u32\n\
               } else {\n\
                 0\n\
               }\n\
             }",
        )]);
        let ts = f.translated_method("lib.rs", "pair");
        assert!(!ts.contains("unsupported("), "{}", ts);
        assert!(!ts.contains("if (false)"), "{}", ts);
        // Each name is read from whichever alternative matched.
        assert!(ts.contains("const l = "), "{}", ts);
        assert!(ts.contains("const p = "), "{}", ts);
        assert!(
            f.messages().iter().all(|m| !m.contains("cannot read back")),
            "and nothing is reported: {:?}",
            f.messages()
        );
    }

    /// D2's other half: where the alternatives really cannot be read back, the
    /// hole stands in the BRANCH and the test is still written — so a value the
    /// pattern does not match runs the `else`, which is what Rust does.
    #[test]
    fn an_unreadable_or_pattern_refuses_in_the_branch_and_still_tests() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct Token { pub n: u32 }\n\
             pub enum Inner { A((Token, Token)), B((Token, Token)) }\n\
             pub fn pick(i: Inner) -> u32 {\n\
               if let Inner::A((a, b)) | Inner::B((b, a)) = i { a.n + b.n } else { 0 }\n\
             }",
        )]);
        let ts = f.translated_method("lib.rs", "pick");
        assert!(!ts.contains("if (unsupported("), "the hole is not the test:\n{}", ts);
        assert!(ts.contains(".is('A')"), "the test is written:\n{}", ts);
        assert!(ts.contains("const a = unsupported("), "{}", ts);
        assert!(ts.contains("const b = unsupported("), "{}", ts);
        assert!(
            f.messages().iter().any(|m| m.contains("cannot read back")),
            "and it says why: {:?}",
            f.messages()
        );
    }
}

#[cfg(test)]
mod slice_pattern_tests {
    use crate::testing::Fixture;

    /// A slice pattern inside a payload member: the LENGTH, and then each
    /// position by its own rule.
    ///
    /// PREMISE CHANGED 2026-09-05 (step 9a slice 2, N4): what stood here was
    /// `if (false)` — an arm written as one that never matches — with the arm's
    /// own bindings written anyway, so `checkedAdd(a, b, 'u32')` named two
    /// identifiers nothing declares.
    #[test]
    fn a_fixed_length_slice_pattern_tests_the_length_and_binds_each_position() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub enum Held { Many(Vec<u32>), None }\n\
             pub fn f(h: Held) -> u32 { \
             match h { Held::Many([a, b]) => a + b, Held::Many(_) => 0, Held::None => 9 } }",
        )]);
        let ts = f.translated_method("lib.rs", "f");
        assert!(ts.contains("v._0.length === 2"), "the length is the test:\n{ts}");
        assert!(ts.contains("const a = v._0[0];"), "{ts}");
        assert!(ts.contains("const b = v._0[1];"), "{ts}");
        assert!(!ts.contains("if (false)"), "{ts}");
        assert!(!ts.contains("unsupported("), "{ts}");
    }

    /// A `..` inside one is the case the port has no lowering for — the
    /// positions after it are counted from the END — so it is a hole, in the
    /// BRANCH, where the arm's own bindings cannot name what nothing declares.
    #[test]
    fn a_slice_pattern_with_a_rest_is_a_hole_in_the_branch() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub enum Held { Many(Vec<u32>), None }\n\
             pub fn f(h: Held) -> u32 { \
             match h { Held::Many([a, ..]) => a, Held::Many(_) => 0, Held::None => 9 } }",
        )]);
        let ts = f.translated_method("lib.rs", "f");
        assert!(ts.contains("unsupported("), "{ts}");
        assert!(!ts.contains("if (false)"), "the refusal is not a test:\n{ts}");
    }
}

#[cfg(test)]
mod const_in_a_payload_tests {
    use crate::testing::Fixture;

    /// A SCREAMING_SNAKE name inside a payload is a CONST: Rust compares the
    /// member against its value, and the arm takes no name out of that
    /// position.
    ///
    /// PREMISE CHANGED 2026-09-05 (step 9a slice 2, N5): `is_irrefutable` and
    /// `binds_nothing` read it as a binding, so the arm was a catch-all that
    /// also owned a value nothing declared. The convention now has one
    /// definition, in `body/pat_shape.rs`, and all three consumers read it.
    #[test]
    fn a_const_inside_a_payload_is_a_comparison_and_binds_nothing() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct Token { pub n: u32 }\n\
             pub const ORIGIN: u32 = 0;\n\
             pub enum Wrap { Held(u32, Token), Empty }\n\
             pub fn f(w: Wrap) -> u32 { \
             match w { Wrap::Held(ORIGIN, t) => { let n = t.n; drop(t); n } \
                       Wrap::Held(_, t) => { drop(t); 2 } Wrap::Empty => 0 } }",
        )]);
        let ts = f.translated_method("lib.rs", "f");
        assert!(ts.contains("v._0 === ORIGIN"), "the member is compared:\n{ts}");
        assert!(!ts.contains("oRIGIN"), "and nothing is bound for it:\n{ts}");
    }
}
