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
}
