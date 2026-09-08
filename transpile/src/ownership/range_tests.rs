//! What a `range.contains(&x)` builds, and who releases it.
//!
//! The range itself is never materialised — the comparison is written from its
//! BOUNDS, which is what lets a range of a width the port cannot count answer
//! at all — so the receiver position writes nothing. What the bounds owe is
//! written where the bounds are.

mod range_bounds {
    use crate::testing::Fixture;

    const KEY: &str = "\
pub struct Key(pub String);\n\
impl PartialEq for Key { fn eq(&self, o: &Key) -> bool { self.0 == o.0 } }\n\
impl PartialOrd for Key {\n\
  fn partial_cmp(&self, o: &Key) -> Option<std::cmp::Ordering> { self.0.partial_cmp(&o.0) }\n\
}\n\
impl Drop for Key { fn drop(&mut self) {} }\n\
pub fn key() -> Key { Key(String::new()) }\n\
";

    fn emitted(rust: &str) -> String {
        let mut fixture = Fixture::build(&[("lib.rs", &format!("{}{}", KEY, rust))]);
        fixture.emitted("lib.rs")
    }

    /// The range is never materialised, so the receiver is written as nothing —
    /// and hoisting nothing wrote `const _t0 = ;`, which a JavaScript engine
    /// will not parse.
    #[test]
    fn the_elided_range_receiver_is_never_hoisted() {
        let ts = emitted("pub fn within(v: Key) -> bool { (key()..key()).contains(&v) }");
        assert!(!ts.contains("= ;"), "nothing is given a name it has no value for:\n{}", ts);
    }

    /// Each bound the expression BUILDS is a temporary Rust drops with the
    /// range, so it is named and released — in Rust's order, start then end.
    #[test]
    fn each_built_bound_is_named_and_released_in_rusts_order() {
        let ts = emitted("pub fn within(v: Key) -> bool { (key()..key()).contains(&v) }");
        assert!(ts.contains("rangeContains(_t0, _t1,"), "both bounds are named:\n{}", ts);
        let start = ts.find("const _t0 = key();").expect("the start is built first");
        let end = ts.find("const _t1 = key();").expect("then the end");
        assert!(start < end, "in Rust's order:\n{}", ts);
        assert!(ts.contains("_t0.drop()") && ts.contains("_t1.drop()"), "both released:\n{}", ts);
    }

    /// A bound that is a PLACE builds nothing and is left where it is.
    #[test]
    fn a_bound_that_names_a_place_is_not_hoisted() {
        let ts = emitted("pub fn within(lo: Key, hi: Key, v: Key) -> bool { (lo..hi).contains(&v) }");
        assert!(ts.contains("rangeContains(lo, hi,"), "the names stand as they are:\n{}", ts);
        assert!(!ts.contains("const _t"), "and nothing is hoisted:\n{}", ts);
    }

    /// GG8: Rust builds the RANGE and evaluates the item after it. The port
    /// lowers a method's arguments before it ever reaches the receiver, so the
    /// item's own temporary stood above both bounds — the emitted text ran the
    /// three side effects in the order I, S, E.
    #[test]
    fn the_item_is_evaluated_after_both_bounds() {
        let ts = emitted("pub fn within() -> bool { (key()..key()).contains(&key()) }");
        let start = ts.find("const _t0 = key();").expect("the start is built first");
        let item = ts.rfind("const _t2 = key();").expect("the item is built last");
        assert!(start < item, "the item stands below both bounds:\n{}", ts);
        assert!(ts.contains("rangeContains(_t0, _t1, false, _t2)"), "in that order:\n{}", ts);
    }

    /// FF2: a bound written as a reference to a CALL was sent through the hoist
    /// TWICE — `_t0 = key()` and then `_t1 = _t0`, a name aliasing a name — so
    /// one value carried two releases and the second was a double drop.
    #[test]
    fn a_bound_already_given_a_name_is_not_given_a_second() {
        let ts = emitted(
            "pub fn look(k: &Key) -> usize { k.0.len() }\n\
             pub fn twice() -> usize { look(&&key()) }",
        );
        assert!(!ts.contains("= _t0;"), "no name aliases a name:\n{}", ts);
        assert_eq!(ts.matches("const _t").count(), 1, "one name for one value:\n{}", ts);
    }

    /// GG9: `vec![v; n]` evaluates v and then n; `Array(n).fill(v)` evaluates n
    /// first, so `vec![value(); count()]` printed the count's side effect where
    /// rustc prints the value's.
    #[test]
    fn a_repeated_vec_evaluates_the_value_before_the_count() {
        let ts = emitted(
            "pub fn value() -> u32 { 7 }\n\
             pub fn count() -> usize { 2 }\n\
             pub fn many() -> Vec<u32> { vec![value(); count()] }",
        );
        let value = ts.find("const _m0 = value();").expect("the value is named");
        let count = ts.find("Array(count())").expect("the count is written where it is");
        assert!(value < count, "the value is evaluated first:\n{}", ts);
    }

    /// FF5: a NESTED `vec![v; n]` writes a `;` of its own, and the repeat form
    /// is read by splitting on the FIRST one — so both halves failed to parse
    /// and the invocation was written through unchanged, leaving
    /// `[vec ! [1i64 ; 2] ; 3]` in the emitted file. Reported, and emitted
    /// anyway: refused now (R12).
    #[test]
    fn a_nested_repeat_is_refused_rather_than_written_through() {
        let ts = emitted("pub fn nested() -> Vec<Vec<u32>> { vec![vec![1u32; 2]; 3] }");
        assert!(ts.contains("unsupported("), "the shape is a hole:\n{}", ts);
        assert!(!ts.contains("vec !"), "and no Rust is left in the output:\n{}", ts);
    }

    /// And a value the port writes as a LITERAL is left where it stands: it
    /// builds nothing, so naming it would only add a line.
    #[test]
    fn a_repeated_literal_is_not_given_a_name() {
        let ts = emitted(
            "pub fn many() -> Vec<String> { vec![String::new(); 2] }\n\
             pub fn numbers() -> Vec<u32> { vec![7u32; 3] }",
        );
        assert!(ts.contains("Array(2).fill('')"), "the string stands:\n{}", ts);
        assert!(ts.contains("Array(3).fill(7)"), "and so does the number:\n{}", ts);
    }
}
