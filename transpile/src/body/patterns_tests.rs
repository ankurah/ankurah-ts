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
    #[test]
    fn a_const_pattern_binds_nothing_and_releases_nothing() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct Point { pub x: i32 }\n             impl Drop for Point { fn drop(&mut self) {} }\n             pub const ORIGIN: Point = Point { x: 0 };\n             pub fn at_origin(p: Point) -> bool { match p { ORIGIN => true, _ => false } }",
        )]);
        let ts = f.translated_method("lib.rs", "at_origin");
        assert!(ts.contains("if (unsupported("), "the test is the hole:\n{ts}");
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
